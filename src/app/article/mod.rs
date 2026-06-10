use slint::ComponentHandle;

pub struct ArticleController;

impl ArticleController {
    pub fn connect(window: &crate::StatusBarWindow) {
        let adapter = window.global::<crate::ArticleAdapter>();
        adapter.on_article_clicked(|| {
            log::info!("[article] clicked");
        });
    }
}
