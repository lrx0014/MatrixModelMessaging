fn main() {
    println!("cargo:rerun-if-changed=migrations");
    for variable in ["DATABASE_URL", "SQLX_OFFLINE", "SQLX_OFFLINE_DIR"] {
        println!("cargo:rerun-if-env-changed={variable}");
    }
}
