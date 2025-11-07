use hydro_test_template as hydro_template;

include!("../../template/hydro/examples/echo_local.rs");

#[test]
fn test() {
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpStream;

    use example_test::run_current_example;

    let mut run = run_current_example!();
    run.read_regex(r"Launched Echo Server");

    let mut tcp_conn = TcpStream::connect("127.0.0.1:4000").unwrap();
    tcp_conn
        .write_all(b"Hello, Hydro!\n")
        .expect("Failed to write to TCP stream");

    let mut lines = BufReader::new(&tcp_conn).lines();
    assert_eq!(lines.next().unwrap().unwrap(), "HELLO, HYDRO!");
}
