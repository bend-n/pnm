fn main() {
    for elem in std::env::args().skip(1) {
        pnm::decode(&std::fs::read(elem).unwrap())
            .unwrap()
            .rgba()
            .show();
    }
}
