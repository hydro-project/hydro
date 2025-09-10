use dfir_rs::dfir_syntax;

fn main() {
    let mut df = dfir_syntax! {
        a = b -> k;

        b = c;
        c = d;
        d = e;
        e = f;
        f = g;
        g = h;
        h = i;
        i = j;
        j = a;

        k = l;
        l = m;
        m = k;
    };
    df.run_available_sync();
}
