use hydro_lang::prelude::*;

/// Takes only a singleton input, so the generated function has no generic parameters.
pub fn consume_greeting<'a>(greeting: Singleton<String, Process<'a, ()>, Bounded>) {
    greeting
        .into_stream()
        .for_each(q!(|greeting| assert_eq!(greeting, "hello")));
}
