/// Terminal color and formatting utilities
pub struct Colors;

impl Colors {
    pub const RESET: &'static str = "\x1b[0m";
    
    // Combined
    pub const BOLD_GREEN: &'static str = "\x1b[1;32m";
    pub const BOLD_YELLOW: &'static str = "\x1b[1;33m";
    pub const BOLD_BLUE: &'static str = "\x1b[1;34m";
}

/// Status tags for output
pub struct Tags;

impl Tags {
    pub fn ok() -> String {
        format!("{}[OK]{}", Colors::BOLD_GREEN, Colors::RESET)
    }
    
    pub fn warn() -> String {
        format!("{}[WARN]{}", Colors::BOLD_YELLOW, Colors::RESET)
    }
    
    pub fn skip() -> String {
        format!("{}[SKIP]{}", Colors::BOLD_YELLOW, Colors::RESET)
    }
    
    pub fn detect() -> String {
        format!("{}[DETECT]{}", Colors::BOLD_BLUE, Colors::RESET)
    }
    
    pub fn interrupt() -> String {
        format!("{}[INTERRUPT]{}", Colors::BOLD_YELLOW, Colors::RESET)
    }
    
    pub fn upload() -> String {
        format!("{}[UPLOAD]{}", Colors::BOLD_GREEN, Colors::RESET)
    }

    pub fn download() -> String {
        format!("{}[DOWNLOAD]{}", Colors::BOLD_BLUE, Colors::RESET)
    }
    
    pub fn file() -> &'static str {
        "[FILE]"
    }
    
    pub fn folder() -> &'static str {
        "[FOLD]"
    }
    
    pub fn clip() -> &'static str {
        "[CLIP]"
    }
    
    pub fn text() -> &'static str {
        "[TEXT]"
    }
    
    pub fn exec() -> &'static str {
        "[EXEC]"
    }
}
