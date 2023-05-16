use std::iter::Iterator;

use crate::lattices::SealedSetOfIndexedValues;
use crate::lattices::VecPrefix;
use crate::{GraphType, Opts};

use hydroflow::hydroflow_syntax;
use hydroflow::lattices::Merge;
use hydroflow::util::ipv4_resolve;
use serde::{Deserialize, Serialize};

// Quotient Lattice:
// Ord x Option<Ord>

// Merge function:
// (8, None) + (9, None) = (9, None)
// (9, None) + (10, Some(10)) = (10, Some(10))

// “Out of spec” cases:
// above top: (11, None) + (10, Some(10)) = (10, Some(10)) (FINE)
// multiple tops: (9, Some(9)) + (10, Some(10)) = ErrTop       (SPECIAL)

// Totally Ordered Function Lattice f:N -> T
// f is a subset of N x T such that for each i in L, there is exactly 1 t in T
// such that (i, t) is in the subset.
// merge (i1, t1), (i2, t2) = (max(i1, i2), t1 if i1 > i2 else t2) is well defined on a TOFL
// observed violations of the functional dependency are dynamic type errors: we're merging across two TOFLs.

// Quotiented Totally Ordered Function Lattice qtof:L -> T
// L a prefix of N "maxing out" at Top_L
// i.e. [Top_L..\infty] is a single equivalence class
// and by guarantee of function (Top_L, t) => (Top_L + k, t) for all k in N

// monotone function seal_Top from qtof(L,T) to eqtof(L, Option<e>, T)
// seal_Top((l, t)) = ((l, None if l < Top else Some(Top), t))

// seal_Top is a morphism:
// seal_Top((l1, t1) + (l2, t2)) =? seal_Top((l1, t1)) + seal_Top((l2, t2))
// seal_Top((max(l1, l2), t1 if l1 > l2 else t2)) =? seal_Top((l1, t1)) + seal((l2, t2))
// <=>
// (max(l1, l2), Some(max(l1, l2)) if max(l1, l2) == Top else None, t1 if l1 > l2 else t2)
//      =? (l1, Some(l1) if l1 == Top else None, t1) + (l2, Some(l2) if l2 == Top else None, t2)
//      ==  (max(l1, l2), Some(max(l1, l2)) if max(l1, l2) == Top else None, t1 if l1 > l2 else t2)
// //

// (8, None, {4}) + (9, None, {5}) = (9, None, {4,5})
// (10, Some(10), {7}) + (9, None, {4, 5}) = (10, Some(10), {4,5,7})
// (10, Some(10), {4,5,7}) + (11, Some(10), {8}) = (10, Some(10), {4,5,7})

#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq, Hash)]
enum ClientClass {
    Basic,
    Prime,
}

#[derive(PartialEq, Eq, Clone, Serialize, Deserialize, Debug)]
struct LineItem {
    name: String,
    qty: i16,
}

#[derive(PartialEq, Eq, Clone, Serialize, Deserialize, Debug)]
struct ClLineItem {
    client: usize,
    li: LineItem,
}

#[derive(PartialEq, Eq, Clone, Serialize, Deserialize, Debug)]
enum Request {
    LineItem { li: LineItem },
    Checkout,
}

// #[derive(PartialEq, Eq, Clone, Serialize, Deserialize, Debug)]
// struct ClRequest {
//     client: usize,
//     req: Request,
// }
type ReqSsiv = SealedSetOfIndexedValues<Request>;
// type ClliSsiv = SealedSetOfIndexedValuesRepr<ClLineItem>;
// type ClReqSsiv = (usize, SealedSetOfIndexedValuesRepr<Request>);
type ReqBpsl = VecPrefix<Request>;
pub(crate) async fn run_client(opts: Opts) {
    // server_addr is required for client
    let addr1 = ipv4_resolve("localhost:23456").unwrap();
    let addr2 = ipv4_resolve("localhost:23457").unwrap();
    let addr3 = ipv4_resolve("localhost:23458").unwrap();
    let out_addr = ipv4_resolve("localhost:23459").unwrap();
    let gossip_addr = ipv4_resolve("localhost:23460").unwrap();

    let (carts_out, carts_in, _) = hydroflow::util::bind_udp_bytes(addr1).await;
    let (reqs_out, reqs_in, _) = hydroflow::util::bind_udp_bytes(addr1).await;
    let (basic_out, basic_in, _) = hydroflow::util::bind_udp_bytes(addr1).await;
    let (prime_out, prime_in, _) = hydroflow::util::bind_udp_bytes(addr2).await;
    let (out, _, _) = hydroflow::util::bind_udp_bytes(out_addr).await;
    let (gossip_out, gossip_in, _) = hydroflow::util::bind_udp_bytes(gossip_addr).await;

    let bpsl_bot: fn() -> ReqBpsl = Default::default;
    let ssiv_bot: fn() -> ReqSsiv = Default::default;

    let bpsl_merge = <ReqBpsl as Merge<ReqBpsl>>::merge;
    let ssiv_merge = <ReqSsiv as Merge<ReqSsiv>>::merge;
    type ReqSsivLattice = ReqSsiv;
    type ReqBpslLattice = ReqBpsl;

    let apple: String = "apple".to_string();
    let banana: String = "banana".to_string();
    let ferrari: String = "ferrari".to_string();
    let potato: String = "potato".to_string();

    let basic_vec: [ClLineItem; 4] = [
        ClLineItem {
            client: 1,
            li: LineItem {
                name: apple.clone(),
                qty: 1,
            },
        },
        ClLineItem {
            client: 1,
            li: LineItem {
                name: banana.clone(),
                qty: 6,
            },
        },
        ClLineItem {
            client: 2,
            li: LineItem {
                name: apple.clone(),
                qty: 1,
            },
        },
        ClLineItem {
            client: 2,
            li: LineItem {
                name: apple.clone(),
                qty: -1,
            },
        },
    ];
    let vec_prime: [ClLineItem; 2] = [
        ClLineItem {
            client: 100,
            li: LineItem {
                name: potato.clone(),
                qty: 1,
            },
        },
        ClLineItem {
            client: 100,
            li: LineItem {
                name: ferrari.clone(),
                qty: 1,
            },
        },
    ];

    let basic = basic_vec.into_iter();
    let prime = vec_prime.into_iter();
    let shopping = basic
        .clone()
        .chain(prime.clone())
        .map(|clli| (clli.client, clli.li));

    let client_class = [
        (1, ClientClass::Basic),
        (2, ClientClass::Basic),
        (100, ClientClass::Prime),
    ]
    .into_iter();

    let basic_req_vec1: [Request; 3] = [
        Request::LineItem {
            li: LineItem {
                name: apple.clone(),
                qty: 1,
            },
        },
        Request::LineItem {
            li: LineItem {
                name: banana,
                qty: 6,
            },
        },
        Request::Checkout,
    ];
    // let basic_req1 = basic_req_vec1.into_iter();

    let basic_req_vec2: [Request; 3] = [
        Request::LineItem {
            li: LineItem {
                name: apple.clone(),
                qty: 1,
            },
        },
        Request::LineItem {
            li: LineItem {
                name: apple,
                qty: -1,
            },
        },
        Request::Checkout,
    ];

    let prime_req_vec: [Request; 3] = [
        Request::LineItem {
            li: LineItem {
                name: potato,
                qty: 1,
            },
        },
        Request::LineItem {
            li: LineItem {
                name: ferrari,
                qty: 1,
            },
        },
        Request::Checkout,
    ];

    let server_addrs = vec![addr1, addr2, addr3].into_iter();

    fn ssiv_wrap<'a>(it: impl 'a + Iterator<Item = Request>) -> impl 'a + Iterator<Item = ReqSsiv> {
        it.scan(false, |checked_out, item| {
            if *checked_out {
                None
            } else {
                *checked_out = matches!(item, Request::Checkout);
                Some(item)
            }
        })
        .enumerate()
        .map(|(idx, item)| match item {
            Request::LineItem { li } => ReqSsiv {
                set: std::iter::once((idx, Request::LineItem { li })).collect(),
                seal: None,
            },
            Request::Checkout { .. } => ReqSsiv {
                set: Default::default(),
                seal: Some(idx + 1),
            },
        })
    }

    fn bpsl_wrap<'a>(it: impl 'a + Iterator<Item = Request>) -> impl 'a + Iterator<Item = ReqBpsl> {
        let mut it = it.enumerate().peekable();
        let mut last: Vec<Request> = Default::default();
        std::iter::from_fn(move || {
            it.next().map(|(idx, x): (usize, Request)| {
                last.push(x);
                ReqBpsl {
                    vec: last.clone(),
                    seal: if it.peek().is_some() {
                        None
                    } else {
                        Some(idx + 1)
                    },
                }
            })
        })
    }

    let basic_req_bpsl = bpsl_wrap(basic_req_vec1.clone().into_iter())
        .map(|r| (1, r))
        .chain(bpsl_wrap(basic_req_vec2.clone().into_iter()).map(|r| (2, r)));
    let prime_req_bpsl = bpsl_wrap(prime_req_vec.clone().into_iter()).map(|r| (100, r));
    let shopping_bpsl = bpsl_wrap(basic_req_vec1.clone().into_iter())
        .map(|r| (1, r))
        .chain(bpsl_wrap(basic_req_vec2.clone().into_iter()).map(|r| (2, r)))
        .chain(bpsl_wrap(prime_req_vec.clone().into_iter()).map(|r| (100, r)));
    let basic_req_ssiv = ssiv_wrap(basic_req_vec1.clone().into_iter())
        .map(|r| (1, r))
        .chain(ssiv_wrap(basic_req_vec2.clone().into_iter()).map(|r| (2, r)));
    let prime_req_ssiv = ssiv_wrap(prime_req_vec.clone().into_iter()).map(|r| (100, r));
    let shopping_ssiv = ssiv_wrap(basic_req_vec1.into_iter())
        .map(|r| (1, r))
        .chain(ssiv_wrap(basic_req_vec2.into_iter()).map(|r| (2, r)))
        .chain(ssiv_wrap(prime_req_vec.into_iter()).map(|r| (100, r)));

    let mut hf = match opts.opt {
        1 => hydroflow_syntax! {
            // ORIG, but mixed clientIds and two customer classes
            source_iter(shopping) -> [0]lookup_class;
            source_iter(client_class) -> [1]lookup_class;
            lookup_class = join()
              -> map(|(client, (li, class))| ((client, class), li))
              -> group_by(Vec::new, Vec::push)
            //   -> map(|m| (m, out_addr)) -> dest_sink_serde(out);
                -> for_each(|((client, class), val)| {
                    println!("({}, {:?}): {:?}", client, class, val);
                });
                // -> null();

        },
        2 => hydroflow_syntax! {
            // BPSL, two customer classes
            source_iter(shopping_bpsl) -> [0]lookup_class;
            source_iter(client_class) -> [1]lookup_class;
            lookup_class = join()
              -> map(|(client, (li, class))| ((client, class), li))
              -> group_by(bpsl_bot, <ReqBpsl as Merge<ReqBpsl>>::merge)
              -> map(|m| (m, out_addr)) -> dest_sink_serde(out);
            //   -> for_each(|m| { println!("{:?}", m) });
            // -> null();
        },
        3 => hydroflow_syntax! {
            // SSIV, two customer classes
            source_iter(shopping_ssiv) -> [0]lookup_class;
            source_iter(client_class) -> [1]lookup_class;
            lookup_class = join()
              -> map(|(client, (li, class))| ((client, class), li))
              -> group_by(ssiv_bot, ssiv_merge)
              -> map(|m| (m, out_addr)) -> dest_sink_serde(out);
            //   -> for_each(|m| { println!("{:?}", m) });
            // -> null();
        },
        4 => hydroflow_syntax! {
            // push group_by through join
            source_iter(shopping_ssiv) -> group_by(ssiv_bot, ssiv_merge) -> [0]lookup_class;
            source_iter(client_class) -> [1]lookup_class;
            lookup_class = join()
              -> map(|(client, (li, class))| ((client, class), li))
              -> map(|m| (m, out_addr)) -> dest_sink_serde(out);
            //   -> for_each(|m| { println!("{:?}", m) });
            // -> null();
        },
        5 => hydroflow_syntax! {
            // Sealed Set Over Network
            source_iter(shopping_ssiv)
              -> map(|pair| (pair, addr1)) -> dest_sink_serde(reqs_out);
            source_stream_serde(reqs_in) -> map(Result::unwrap) -> map(|((client, req), _a): ((usize, ReqSsivLattice), _)| (client, req))
              -> group_by(ssiv_bot, ssiv_merge) -> [0]lookup_class;
            source_iter(client_class) -> [1]lookup_class;
            lookup_class = join()
              -> map(|(client, (li, class))| ((client, class), li))
              -> map(|m| (m, out_addr)) -> dest_sink_serde(out);
            //   -> for_each(|m| { println!("{:?}", m) });
            // -> null();
        },
        6 => hydroflow_syntax! {
            // Client-Side State
            source_iter(shopping_ssiv)
              -> group_by(ssiv_bot, ssiv_merge)
              -> map(|pair| (pair, addr1)) -> dest_sink_serde(carts_out);
            source_stream_serde(carts_in) -> map(Result::unwrap) -> map(|((client, cart), _a): ((usize, ReqSsivLattice), _)| (client, cart))
              -> [0]lookup_class;
            source_iter(client_class) -> [1]lookup_class;
            lookup_class = join()
              -> map(|(client, (li, class))| ((client, class), li))
              -> map(|m| (m, out_addr)) -> dest_sink_serde(out);
            //   -> for_each(|m| { println!("{:?}", m) });
            // -> null();
        },
        7 => hydroflow_syntax! {
            // Replicated Server
            source_iter(shopping_ssiv)
              -> map(|pair| (pair, addr1)) -> dest_sink_serde(carts_out);
            source_stream_serde(carts_in) -> map(Result::unwrap) -> map(|((client, cart), _a): ((usize, ReqSsivLattice), _)| (client, cart))
                -> group_by(ssiv_bot, ssiv_merge) -> [0]lookup_class;
            source_iter(client_class) -> [1]lookup_class;
            lookup_class = join()
              -> map(|(client, (li, class))| ((client, class), li) ) -> tee();
            lookup_class[clients] -> all_in;
            lookup_class[gossip] -> [0]gossip;
            source_iter(server_addrs) -> [1]gossip;
            gossip = cross_join() -> dest_sink_serde(gossip_out);
            source_stream_serde(gossip_in) -> map(Result::unwrap) -> map(|(m, _a): (((usize, ClientClass), ReqSsivLattice), _)| m) -> all_in;
            all_in = merge()
              -> group_by(ssiv_bot, ssiv_merge)
              -> map(|m| (m, out_addr)) -> dest_sink_serde(out);
            //   -> for_each(|m| { println!("{:?}", m) });
             // -> null();


            // source_iter(server_addrs) -> [1]gossip;
            // gossip = cross_join() -> dest_sink_serde(gossip_out);
            // all_in[out] -> map(|m| (m, out_addr)) -> dest_sink_serde(out);
            // gossip_in = source_stream_serde(gossip_in) -> map(|(m, _a): (((usize, ClientClass), ReqSsiv), _)| m) -> tee();
            // gossip_in[0] -> all_groups;
            // gossip_in[1] -> for_each(|m| { println!("{:?}", m) });
        },
        11 => hydroflow_syntax! {
            // ORIG, but mixed clientIds and two customer classes
            source_iter(basic) -> map(|clli| ((clli.client, ClientClass::Basic), clli.li)) -> all_requests;
            source_iter(prime) -> map(|clli| ((clli.client, ClientClass::Prime), clli.li)) -> all_requests;
            all_requests = merge() -> group_by(Vec::new, Vec::push)
            //   -> map(|m| (m, out_addr)) -> dest_sink_serde(out);
                -> for_each(|((client, class), val)| {
                    println!("({}, {:?}): {:?}", client, class, val);
                });
                // -> null();

        },
        12 => hydroflow_syntax! {
            // BPSL, two customer classes
            source_iter(basic_req_bpsl)
              -> map(|(client, bpsl): (usize, ReqBpslLattice)| ((client, ClientClass::Basic), bpsl))
              -> all_requests;
            source_iter(prime_req_bpsl)
              -> map(|(client, bpsl): (usize, ReqBpslLattice)| ((client, ClientClass::Prime), bpsl))
              -> all_requests;
            all_requests = merge() -> group_by(bpsl_bot, bpsl_merge)
            //   -> map(|m| (m, out_addr)) -> dest_sink_serde(out);
              -> for_each(|m| { println!("{:?}", m) });
            // -> null();
        },
        13 => hydroflow_syntax! {
            // Sealed Set, 2 Classes
            source_iter(basic_req_ssiv)
              -> map(|(client, m): (usize, ReqSsiv)| ((client, ClientClass::Basic), m))
              -> all_requests;
            source_iter(prime_req_ssiv)
              -> map(|(client, m): (usize, ReqSsiv)| ((client, ClientClass::Prime), m))
              -> all_requests;
            all_requests = merge() -> group_by(ssiv_bot, ssiv_merge)
            //   -> map(|m| (m, out_addr)) -> dest_sink_serde(out);
              -> for_each(|(client, val)| {
                  println!("{:?}: {:?}", client, val);
            });
            // -> null();
        },
        14 => hydroflow_syntax! {
            // Push GroupBy Thru Merge
           source_iter(basic_req_ssiv)
              -> map(|(client, m): (usize, ReqSsiv)| ((client, ClientClass::Basic), m))
              -> group_by(ssiv_bot, ssiv_merge)
              -> all_requests;
            source_iter(prime_req_ssiv)
              -> map(|(client, m): (usize, ReqSsiv)| ((client, ClientClass::Prime), m))
              -> group_by(ssiv_bot, ssiv_merge)
              -> all_requests;
            all_requests = merge()
                //  -> map(|m| (m, out_addr)) -> dest_sink_serde(out);
              -> for_each(|(client, val)| {
                  println!("{:?}: {:?}", client, val);
            });
            // -> null();
        },
        15 => hydroflow_syntax! {
            // Push GroupBy Thru Map
            source_iter(basic_req_ssiv)
              -> group_by(ssiv_bot, ssiv_merge)
              -> map(|(client, m)| (client, (ClientClass::Basic, m)))
              -> all_groups;
            source_iter(prime_req_ssiv)
              -> group_by(ssiv_bot, ssiv_merge)
              -> map(|(client, m)| (client, (ClientClass::Prime, m)))
              -> all_groups;
            all_groups = merge() -> map(|m| (m, out_addr)) -> dest_sink_serde(out);
            //   -> for_each(|(client, val)| {
            //       println!("{}: {:?}", client, val);
            // });
            // -> null();
        },
        16 => hydroflow_syntax! {
            // Sealed Set Over Network
            source_iter(basic_req_ssiv)
              -> map(|pair| (pair, addr1)) -> dest_sink_serde(basic_out);
            source_iter(prime_req_ssiv)
              -> map(|pair| (pair, addr2)) -> dest_sink_serde(prime_out);
            source_stream_serde(basic_in) -> map(Result::unwrap) -> map(|((client, cart), _a): ((usize, ReqSsiv), _)| (client, cart))
              -> group_by(ssiv_bot, ssiv_merge)
              -> map(|(client, m)| (client, (ClientClass::Basic, m)))
              -> all_groups;
            source_stream_serde(prime_in) -> map(Result::unwrap) -> map(|((client, cart), _a): ((usize, ReqSsiv), _)| (client, cart))
              -> group_by(ssiv_bot, ssiv_merge)
              -> map(|(client, m)| (client, (ClientClass::Prime, m)))
              -> all_groups;
            all_groups = merge() -> map(|m| (m, out_addr)) -> dest_sink_serde(out);
            //   -> for_each(|(client, val)| {
            //       println!("{}: {:?}", client, val);
            // });
            // -> null();
        },
        17 => hydroflow_syntax! {
            // Replicated Server
            source_stream_serde(basic_in) -> map(Result::unwrap) -> map(|((client, cart), _a): ((usize, ReqSsiv), _)| (client, cart))
              -> group_by(ssiv_bot, ssiv_merge)
              -> map(|(client, m)| (client, (ClientClass::Basic, m)))
              -> all_groups;
            source_stream_serde(prime_in) -> map(Result::unwrap) -> map(|((client, cart), _a): ((usize, ReqSsiv), _)| (client, cart))
              -> group_by(ssiv_bot, ssiv_merge)
              -> map(|(client, m)| (client, (ClientClass::Prime, m)))
              -> all_groups;
            all_groups = merge()
              -> group_by(|| (None, ssiv_bot()),
              |s: &mut (Option<ClientClass>, ReqSsiv), c: (ClientClass, ReqSsiv)| {
              if s.0.is_none() {s.0 = Some(c.0);}
              ssiv_merge(&mut s.1, c.1);
              }) -> tee();
            all_groups[gossip] -> [0]gossip;
            source_iter(server_addrs) -> [1]gossip;
            gossip = cross_join() -> dest_sink_serde(gossip_out);
            all_groups[out] -> map(|m| (m, out_addr)) -> dest_sink_serde(out);
            source_stream_serde(gossip_in) -> map(Result::unwrap) -> map(|(m, _a)| m) -> all_groups;
        },
        // 8 => hydroflow_syntax! {
        //     // ORIG, but mixed clients and separate channel for Checkout
        //     source_iter(basic) -> map(|clli| (clli.client, clli.li)) -> all_shopping;
        //     source_iter(clli_prime_stream) -> map(|clli| (clli.client, clli.li)) -> all_shopping;
        //     all_shopping = merge()
        //         -> group_by(Vec::new, |s: &mut Vec<LineItem>, c: LineItem| s.push(c))
        //         -> [0]orders;
        //     checkouts = source_iter(clco_stream) -> all_checkouts;
        //     prime_checkouts = source_iter(clco_prime_stream) -> all_checkouts;
        //     all_checkouts = merge() -> map(|client| (client, ())) -> [1]orders;
        //     orders = join() -> map(|(client, (lineitems, _))| (client, lineitems))
        //         // -> map(|m| (m, out_addr)) -> dest_sink_serde(out);
        //         -> for_each(|(client, lineitems)| {
        //             println!("{}: {:?}", client, lineitems);
        //         });
        //     //  -> null();
        // },
        // 9 => hydroflow_syntax! {
        //     // Sealed Set Over Network, but mixed clients and separate channel for Checkout
        //     source_stream_serde(basic_in)
        //       -> map(|((client, m), _a): ((usize, ReqSsiv), _)| (client, m))
        //       -> all_shopping;
        //     source_stream_serde(prime_in)
        //       -> map(|((client, m), _a): ((usize, ReqSsiv), _)| (client, m))
        //       -> all_shopping;
        //     all_shopping = merge()
        //         -> group_by(ssiv_bot, ssiv_merge)
        //         -> [0]orders;
        //     all_checkouts = source_stream_serde(client3_in) -> map(|(client, _a): (usize, _)| (client, ())) -> [1]orders;
        //     orders = join() -> map(|(client, (lineitems, _))| (client, lineitems))
        //         // -> map(|m| (m, out_addr)) -> dest_sink_serde(out);
        //         -> for_each(|(client, lineitems)| {
        //             println!("{}: {:?}", client, lineitems);
        //         });
        //     //  -> null();
        // },
        // 0 => hydroflow_syntax! {
        //     // ORIG: non-monotonic accumulation of chars
        //     // runs til the "end of time"
        //     // Order-sensitive group-by function
        //     basic = source_iter(li_stream1) -> map(|c| (1, c)) -> all_requests;
        //     prime = source_iter(li_stream2) -> map(|c| (2, c)) -> all_requests;
        //     all_requests = merge()
        //         -> group_by(Vec::new, |s: &mut Vec<LineItem>, c: LineItem| s.push(c))
        //         // -> map(|m| (m, out_addr)) -> dest_sink_serde(out);
        //         -> for_each(|(client, val)| {
        //             println!("{}: {:?}", client, val);
        //         });
        //         // -> null();

        // },
        // 1 => hydroflow_syntax! {
        //     // PREFIX LATTICE:
        //     // Can output each client when it finishes
        //     // Order-insensitive but large state overhead
        //     basic = source_iter(bpsl_wrap(basic_req1)) -> map(|x| (1, x))
        //       -> all_requests;
        //     prime = source_iter(bpsl_wrap(basic_req2)) -> map(|x| (2, x))
        //       -> all_requests;
        //     all_requests = merge()
        //         -> group_by(bpsl_bot, <RecBpsl as Merge<RecBpsl>>::merge)
        //         // -> map(|m| (m, out_addr)) -> dest_sink_serde(out);
        //         -> for_each(|(client, val)| {
        //             println!("{}: {:?}", client, val);
        // });
        // },
        // 2 => hydroflow_syntax! {
        //     // ReqSsiv LATTICE: basic
        //     // Can output each client when it finishes
        //     // Order-insensitive and cheap
        //     basic = source_iter(ssiv_wrap(basic_req1)) -> map(|x| (1, x))
        //       -> all_requests;
        //     prime = source_iter(ssiv_wrap(basic_req2)) -> map(|x| (2, x))
        //       -> all_requests;
        //     all_requests = merge()
        //         -> group_by(|| ssiv_bot(),
        //                     ssiv_merge)
        //         // -> map(|m| (m, out_addr)) -> dest_sink_serde(out);
        //         -> for_each(|(client, val)| {
        //                     println!("{}: {:?}", client, val);
        //         });
        //         // -> null();
        // },
        // 3 => hydroflow_syntax! {
        //     // Push Aggregation Through Merge
        //     basic = source_iter(ssiv_wrap(basic_req1)) -> map(|x| (1, x)) -> reduce(|(client, stored), (_prime, q)| {
        //                 (client, ssiv_merge_owned(stored, q))
        //             }) -> all_folds;
        //     prime = source_iter(ssiv_wrap(basic_req2)) -> map(|x| (2, x)) -> reduce(|(client, stored), (_prime, q)| {
        //                 (client, ssiv_merge_owned(stored, q))
        //             }) -> all_folds;
        //     all_folds = merge() -> map(|m| (m, out_addr)) -> dest_sink_serde(out);
        //     // -> for_each(|(client, val)| {
        //     //     println!("{}: {:?}", client, val);
        //     // });
        //        // -> null();
        // },
        // 4 => hydroflow_syntax! {
        //     // Push Reduce through Map
        //     basic = source_iter(ssiv_wrap(basic_req1))
        //     -> reduce(ssiv_merge_owned)
        //     -> map(|q| (1, q)) -> all_folds;
        //     prime = source_iter(ssiv_wrap(basic_req2))
        //     -> reduce(ssiv_merge_owned)
        //     -> map(|q| (1, q)) -> all_folds;
        //     all_folds = merge() -> map(|m| (m, out_addr)) -> dest_sink_serde(out);
        //     // -> for_each(|(client, val)| { println!("{}: {:?}", client, val); });
        //    // -> null();
        // },
        // 5 => hydroflow_syntax! {
        //     // Sealed Set Over Network
        //     basic = source_iter(ssiv_wrap(basic_req1)) -> map(|m| (m, addr1)) -> dest_sink_serde(basic_out);
        //     prime = source_iter(ssiv_wrap(basic_req2)) -> map(|m| (m, addr2)) -> dest_sink_serde(prime_out);
        //     source_stream_serde(basic_in) -> map(|(m, _a): (ReqSsiv, _)| m)
        //       -> reduce(ssiv_merge_owned) -> map(|q| (1, q)) -> all_folds;
        //     source_stream_serde(prime_in) -> map(|(m, _a): (ReqSsiv, _)| m)
        //       -> reduce(ssiv_merge_owned) -> map(|q| (1, q)) -> all_folds;
        //     all_folds = merge() -> map(|m| (m, out_addr)) -> dest_sink_serde(out);
        //     //   -> for_each(|(client, val)| {
        //     //       println!("{}: {:?}", client, val);
        //     // });
        //     // -> null();
        // },
        // 6 => hydroflow_syntax! {
        //     // Replicated Server
        //     source_stream_serde(basic_in) -> map(|(m, _a): (ReqSsiv, _)| m)
        //       -> reduce(ssiv_merge_owned) -> map(|q| (1, q)) -> all_folds;
        //     source_stream_serde(prime_in) -> map(|(m, _a): (ReqSsiv, _)| m)
        //       -> reduce(ssiv_merge_owned) -> map(|q| (1, q)) -> all_folds;
        //     source_stream_serde(gossip_in)
        //       -> map(|((client, q), _a): ((usize, ReqSsiv), _)| (client, q)) -> all_folds;
        //     all_folds = merge() -> group_by(|| ssiv_bot(), ssiv_merge) -> map(|m| (m, out_addr)) -> tee();
        //     all_folds[gossip] -> dest_sink_serde(gossip_out);
        //     all_folds[out] -> dest_sink_serde(out);
        //     //   -> for_each(|(client, val)| {
        //     //       println!("{}: {:?}", client, val);
        //     // });
        //     // -> null();
        // },
        // 7 => hydroflow_syntax! {
        //     // SHOPPING CART
        //     basic = source_iter(ssiv_wrap(basic_req1)) -> map(|x| (1, x))
        //       -> all_requests;
        //     prime = source_iter(ssiv_wrap(basic_req2)) -> map(|x| (2, x))
        //       -> all_requests;
        //     all_requests = merge()
        //         -> group_by(|| ssiv_bot(),
        //              ssiv_merge)
        //         -> map(|m| (m, out_addr)) -> dest_sink_serde(out);
        // //         -> for_each(|(client, val)| {
        // //             println!("{}: {:?}", client, val);
        // // });
        // },
        // 10 => hydroflow_syntax! {
        //     // ClReqSsiv LATTICE:
        //     // Can output each client when it finishes
        //     // Order-insensitive and cheap
        //     source_iter(clli_ssiv_wrap(basic)) -> map(|(cls, eos)| cls) -> flatten() -> map(|(client, li)| (client, li))
        //         -> all_shopping;
        //     shopping_prime = source_iter(clli_ssiv_wrap(clli_prime_stream)) -> map(|(cls, eos)| cls) -> flatten() -> map(|(client, li)| (client, li))
        //         -> all_shopping;
        //     all_merge()
        //         -> group_by(|| clli_ssiv_bot.clone(), <ClliSsiv as Merge<ClliSsiv>>::merge)
        //         -> [0]orders;
        //     checkouts = source_iter(clco_stream) -> all_checkouts;
        //     prime_checkouts = source_iter(clco_prime_stream) -> all_checkouts;
        //     all_checkouts = merge() -> map(|client| (client, ())) -> [1]orders;
        //     orders = join() -> map(|(client, (lineitems, _))| (client, lineitems))
        //         // -> map(|m| (m, out_addr)) -> dest_sink_serde(out);
        //         -> for_each(|(client, lineitems)| {
        //             println!("{}: {:?}", client, lineitems);
        //         });
        //     //  -> null();
        // },
        // 9 => hydroflow_syntax! {
        //     // ReqSsiv LATTICE: basic
        //     // Can output each client when it finishes
        //     // Order-insensitive and cheap
        //     inbound = source_iter(clreq_stream) // -> map(|(client, req_ssiv)| (client, req_ssiv)) -> flatten()
        //       -> all_requests;
        //     all_requests = group_by(|| ssiv_bot.clone(),
        //                     ssiv_merge)
        //         // -> map(|m| (m, out_addr)) -> dest_sink_serde(out);
        //         -> for_each(|(client, val)| {
        //                     println!("{}: {:?}", client, val);
        //         });
        //         // -> null();
        // },
        // 8 => hydroflow_syntax! {
        //     // VecPrefix with agg pushed thru merge/map
        //     basic = source_iter(bpsl_wrap(chars10))
        //         -> reduce(<VecPrefixRepr as Merge<VecPrefixRepr>>::merge_owned)
        //         -> map(|x| (1, x))
        //         -> all_folds;
        //     prime = source_iter(bpsl_wrap(chars5))
        //         -> reduce(<VecPrefixRepr as Merge<VecPrefixRepr>>::merge_owned)
        //         -> map(|x| (2, x))
        //         -> all_folds;
        //     all_folds = merge()
        //     -> for_each(|(client, val)| {
        //         println!("{}: {:?}", client, val);
        //     });
        // // -> null();
        //     },

        // 4 => hydroflow_syntax! {
        //     // QTOFL WITH EXPANDED GROUP_BY: basic: (9, Some(9), 106)
        //     basic = source_iter(qtofl_wrap(chars10)) -> map(|x| (1, x)) -> all_requests;
        //     prime = source_iter(qtofl_wrap(chars5)) -> map(|x| (2, x)) -> all_requests;
        //     all_requests = merge() -> groupx;
        //     groupx = demux(|(client, qtofl), var_args!(basic_out, prime_out, err)|
        //         match client {
        //             1 => basic_out.give((client, qtofl)),
        //             2 => prime_out.give((client, qtofl)),
        //             _ => err.give((client, qtofl))
        //         });
        //     groupx[basic_out] -> reduce(|(client, stored), (_prime, q)| {
        //         (client, <QtoflRepr<usize, char> as Merge<QtoflRepr<usize, char>>>::merge_owned(stored, q))
        //     })
        //     -> all_folds;
        //     groupx[prime_out] -> reduce(|(client, stored), (_prime, q)| {
        //         (client, <QtoflRepr<usize, char> as Merge<QtoflRepr<usize, char>>>::merge_owned(stored, q))
        //     })
        //     -> all_folds;
        //     groupx[err] -> null();
        //     all_folds = merge()
        //     -> for_each(|(client, val)| {
        //                 println!("{}: {:?}", client, val);
        //             });
        //     // -> null();
        // },
        // 5 => hydroflow_syntax! {
        //     // Remove merge->demux
        //     basic = source_iter(qtofl_wrap(chars10))
        //         -> reduce(<QtoflRepr<usize, char> as Merge<QtoflRepr<usize, char>>>::merge_owned)
        //         -> map(|x| (1, x))
        //         -> all_folds;
        //     prime = source_iter(qtofl_wrap(chars5))
        //         -> reduce(<QtoflRepr<usize, char> as Merge<QtoflRepr<usize, char>>>::merge_owned)
        //         -> map(|x| (2, x))
        //         -> all_folds;
        //     all_folds = merge()
        //     -> for_each(|(client, val)| {
        //         println!("{}: {:?}", client, val);
        //     });
        // // -> null();
        //     },
        // 7 => hydroflow_syntax! {
        //     // Punctuated Stream style
        //     basic = source_iter(chars10.map(Some).chain([None].into_iter()))
        //         -> map(|c| (1, if let Some(ch) = c {Some(ch)} else {None as Option<char>})) -> all_requests;
        //     prime = source_iter(chars5.map(Some).chain([None].into_iter()))
        //         -> map(|c| (2, if let Some(ch) = c {Some(ch)} else {None as Option<char>})) -> all_requests;
        //     all_requests = merge()
        //         -> group_by(String::new, |s: &mut String, c: Option<char>| if let Some(ch) = c {s.push(ch)} else {finalize(s);})
        //         -> for_each(|(client, val)| {
        //             println!("{}: {:?}", client, val);
        //         });
        //         // -> null();
        // },
        _ => panic!("Invalid opt number"),
    };

    // optionally print the dataflow graph
    if let Some(graph) = opts.graph {
        let serde_graph = hf
            .meta_graph()
            .expect("No graph found, maybe failed to parse.");
        match graph {
            GraphType::Mermaid => {
                println!("{}", serde_graph.to_mermaid());
            }
            GraphType::Dot => {
                println!("{}", serde_graph.to_dot())
            }
            GraphType::Json => {
                unimplemented!();
            }
        }
    }
    hf.run_available();
    // hf.run_async().await.unwrap();
}
