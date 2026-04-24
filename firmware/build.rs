fn main() {
    // both board targets use esp-idf-svc; embuild wires up the linker args
    #[cfg(any(feature = "board-cyd", feature = "board-qemu"))]
    embuild::espidf::sysenv::output();
}
