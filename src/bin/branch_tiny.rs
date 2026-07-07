use sts_simulator::runtime::branch::OwnerAuditRuntime;

fn main() {
    if let Err(err) = OwnerAuditRuntime::run_cli() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}
