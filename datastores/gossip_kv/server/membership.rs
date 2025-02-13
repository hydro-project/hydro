use std::sync::OnceLock;

use gossip_kv::membership::MemberId;
use rand::distributions::Distribution;
use rand::{thread_rng, Rng};

/// This is a simple distribution that generates a random lower-case alphanumeric
struct LowercaseAlphanumeric;

impl Distribution<char> for LowercaseAlphanumeric {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> char {
        let choices = b"abcdefghijklmnopqrstuvwxyz0123456789";
        choices[rng.gen_range(0..choices.len())] as char
    }
}

/// Gets a name for the current process.
pub fn member_name(random_suffix_len: usize) -> &'static MemberId {
    static MEMBER_NAME: OnceLock<MemberId> = OnceLock::new();
    MEMBER_NAME.get_or_init(|| {
        let hostname = hostname::get().unwrap().to_str().unwrap().to_string();

        if random_suffix_len > 0 {
            let suffix: String = thread_rng()
                .sample_iter(&LowercaseAlphanumeric)
                .take(4)
                .collect();
            format!("{}-{}", hostname, suffix)
        } else {
            hostname
        }
    })
}
