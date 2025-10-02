use std::cell::RefCell;
use std::time::Duration;

use dfir_rs::compiled::pull::{JoinState, SymmetricHashJoin};
use dfir_rs::lang::collections::Iter;
use dfir_rs::scheduled::graph::Dfir;
use dfir_rs::scheduled::graph_ext::GraphExt;
use dfir_rs::scheduled::handoff::VecHandoff;
use dfir_rs::var_expr;
use pusherator::{InputBuild, IteratorToPusherator, PusheratorBuild};
use rand::Rng;

mod people;

// const TRANSMISSIBLE_DURATION: Duration = Duration::from_secs(14 * 24 * 3600);
const TRANSMISSIBLE_DURATION: usize = 14;

fn main() {
    type Pid = &'static str;
    type Name = &'static str;
    type Phone = &'static str;
    type DateTime = usize;

    let mut df = Dfir::new();

    let (contacts_send, contacts_recv) =
        df.make_edge::<_, VecHandoff<(Pid, Pid, DateTime)>>("contacts");
    let contacts_send = df.add_channel_input("contacts input", contacts_send);

    let (diagnosed_send, diagnosed_recv) =
        df.make_edge::<_, VecHandoff<(Pid, (DateTime, DateTime))>>("diagnosed");
    let diagnosed_send = df.add_channel_input("diagnosed input", diagnosed_send);

    let (people_send, people_recv) = df.make_edge::<_, VecHandoff<(Pid, (Name, Phone))>>("people");
    let people_send = df.add_channel_input("people input", people_send);

    let (loop_send, loop_recv) = df.make_edge::<_, VecHandoff<(Pid, DateTime)>>("loop");
    let (notifs_send, notifs_recv) = df.make_edge::<_, VecHandoff<(Pid, DateTime)>>("notifs");

    type MyJoinState = RefCell<JoinState<&'static str, (usize, usize), (&'static str, usize)>>;
    let state_handle = df.add_state(MyJoinState::default());

    df.add_subgraph(
        "main",
        var_expr!(contacts_recv, diagnosed_recv, loop_recv),
        var_expr!(notifs_send, loop_send),
        move |context,
              var_expr!(contacts_recv, diagnosed_recv, loop_recv),
              var_expr!(notifs_send, loop_send)| {
            let looped = loop_recv
                .take_inner()
                .into_iter()
                .map(|(pid, t)| (pid, (t, t + TRANSMISSIBLE_DURATION)));

            let exposed = diagnosed_recv.take_inner().into_iter().chain(looped);

            let contacts = contacts_recv
                .take_inner()
                .into_iter()
                .flat_map(|(pid_a, pid_b, t)| vec![(pid_a, (pid_b, t)), (pid_b, (pid_a, t))]);

            let mut join_state = context.state_ref(state_handle).borrow_mut();
            let join_exposed_contacts = SymmetricHashJoin::new(exposed, contacts, &mut *join_state);
            let new_exposed = join_exposed_contacts.filter_map(
                |(_pid_a, ((t_from, t_to), (pid_b, t_contact)))| {
                    if t_from < t_contact && t_contact <= t_to {
                        Some((pid_b, t_contact))
                    } else {
                        None
                    }
                },
            );

            let pivot = new_exposed
                .pull_to_push()
                .tee(
                    InputBuild::new().for_each(|exposed_person: (Pid, DateTime)| {
                        // Notif push.
                        notifs_send.give(Some(exposed_person));
                    }),
                )
                .for_each(|exposed_person: (Pid, DateTime)| {
                    // Loop push.
                    loop_send.give(Some(exposed_person));
                });

            pivot.run();
        },
    );

    let mut people_exposure = Default::default();

    df.add_subgraph(
        "join people and notifs",
        var_expr!(people_recv, notifs_recv),
        var_expr!(),
        move |_ctx, var_expr!(peoples, exposures), ()| {
            let exposures = exposures.take_inner().into_iter();
            let peoples = peoples.take_inner().into_iter();

            let joined = SymmetricHashJoin::new(peoples, exposures, &mut people_exposure);

            let pivot = joined
                .pull_to_push()
                .for_each(|(_pid, ((name, phone), exposure))| {
                    println!(
                        "[{}] To {}: Possible Exposure at t = {}",
                        name, phone, exposure
                    );
                });

            pivot.run();
        },
    );

    let all_people = people::get_people();

    let inner = all_people.clone();
    std::thread::spawn(move || {
        people_send.give(Iter(inner.into_iter()));
        people_send.flush();
    });

    std::thread::spawn(move || {
        let mut t = 0;
        let mut rng = rand::thread_rng();
        loop {
            t += 1;
            match rng.gen_range(0..2) {
                0 => {
                    // New contact.
                    if all_people.len() >= 2 {
                        let p1 = rng.gen_range(0..all_people.len());
                        let p2 = rng.gen_range(0..all_people.len());
                        if p1 != p2 {
                            contacts_send.give(Some((all_people[p1].0, all_people[p2].0, t)));
                            contacts_send.flush();
                        }
                    }
                }
                1 => {
                    // Diagnosis.
                    if !all_people.is_empty() {
                        let p = rng.gen_range(0..all_people.len());
                        diagnosed_send
                            .give(Some((all_people[p].0, (t, t + TRANSMISSIBLE_DURATION))));
                        diagnosed_send.flush();
                    }
                }
                _ => unreachable!(),
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    });

    loop {
        df.run_available();
    }
}
