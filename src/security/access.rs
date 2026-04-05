pub fn check_access(role: &str) -> bool {
    match role {
        "admin" => true,
        "service" => true,
        _ => false,
    }
}
