fn main() {
    #[cfg(windows)]
    {
        let ico = std::path::Path::new("webroot/img/jukeboks.ico");
        if ico.exists() {
            let mut res = winres::WindowsResource::new();
            res.set_icon("webroot/img/jukeboks.ico");
            let _ = res.compile();
        }
    }
}
