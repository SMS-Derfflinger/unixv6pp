fn add_cfg(cfg: &str) {
    println!("cargo::rustc-check-cfg=cfg({cfg})");
}

pub fn main() {
    add_cfg("trace_alloc");
}
