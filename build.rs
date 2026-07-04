fn main() {
    let mut res = winres::WindowsResource::new();
    res.set_icon("Blur_0.ico");
    res.compile().unwrap();
}
