use ratatui::style::Color;

// Color Palette (matching Go version exactly)
pub const GREEN: Color = Color::Rgb(4, 181, 117);     // #04B575
pub const BLUE: Color = Color::Rgb(46, 125, 225);     // #2E7DE1
pub const PURPLE: Color = Color::Rgb(167, 139, 250);  // #A78BFA
pub const GRAY: Color = Color::Rgb(107, 114, 128);    // #6B7280
pub const WHITE: Color = Color::Rgb(255, 255, 255);   // #FFFFFF
pub const DARK: Color = Color::Rgb(31, 41, 55);       // #1F2937
pub const YELLOW: Color = Color::Rgb(245, 158, 11);   // #F59E0B
pub const RED: Color = Color::Rgb(239, 68, 68);       // #EF4444
pub const CYAN: Color = Color::Rgb(0, 217, 255);      // #00D9FF
pub const LIGHT_BLUE: Color = Color::Rgb(78, 167, 255); // #4EA7FF
pub const DARK_BG: Color = Color::Rgb(45, 55, 72);    // #2D3748
pub const MODAL_BG: Color = Color::Rgb(26, 27, 38);   // #1A1B26
pub const DIM_BLUE: Color = Color::Rgb(86, 95, 137);  // #565f89
pub const TERM_GRAY: Color = Color::Rgb(168, 168, 168); // #A8A8A8
pub const CHECKBOX_DIM: Color = Color::Rgb(75, 85, 99); // #4B5563

/// Splash screen color cycle
pub const SPLASH_COLORS: [Color; 6] = [
    BLUE,
    LIGHT_BLUE,
    PURPLE,
    CYAN,
    GREEN,
    BLUE,
];

/// Spinner animation frames
pub const SPINNER_FRAMES: [&str; 10] = [
    "⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏",
];

pub const SPARK_ART: &str = r#"
   _____ ____  ___  ____  __ __
  / ___// __ \/   |/ __ \/ //_/
  \__ \/ /_/ / /| / /_/ / ,<
 ___/ / ____/ ___ / _, _/ /| |
/____/_/   /_/  |/_/ |_/_/ |_|
"#;

pub const VERSION: &str = "v0.7.0";
