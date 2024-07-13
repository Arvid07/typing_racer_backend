use serde::Serialize;
use strum_macros::EnumIter;

#[derive(Debug, Clone, Copy, Serialize, EnumIter)]
pub enum UserColor {
    AliceBlue,
    AntiqueWhite,
    Aqua,
    Aquamarine,
    Azure,
    Beige,
    Bisque,
    BlanchedAlmond,
    Blue,
    BlueViolet,
    BurlyWood,
    CadetBlue,
    Chartreuse,
    Chocolate,
    Coral,
    CornflowerBlue,
    Cornsilk,
    Crimson,
    Cyan,
    DarkCyan,
    DarkGoldenRod,
    DarkKhaki,
    DarkOrange,
    DarkSalmon,
    DarkSeaGreen,
    DarkTurquoise,
    DarkViolet,
    DeepPink,
    DeepSkyBlue,
    DodgerBlue,
    FireBrick,
    FloralWhite,
    ForestGreen,
    Fuchsia,
    Gainsboro,
    GhostWhite,
    Gold,
    GoldenRod,
    GreenYellow,
    HoneyDew,
    HotPink,
    IndianRed,
    Ivory,
    Khaki,
    Lavender,
    LavenderBlush,
    LawnGreen,
    LemonChiffon,
    LightBlue,
    LightCoral,
    LightCyan,
    LightGoldenRodYellow,
    LightGreen,
    LightPink,
    LightSalmon,
    LightSeaGreen,
    LightSkyBlue,
    LightSteelBlue,
    LightYellow,
    Lime,
    LimeGreen,
    Linen,
    Magenta,
    MediumAquaMarine,
    MediumSpringGreen,
    MediumTurquoise,
    MediumVioletRed,
    MintCream,
    MistyRose,
    Moccasin,
    NavajoWhite,
    OldLace,
    OliveDrab,
    Orange,
    OrangeRed,
    Orchid,
    PaleGoldenRod,
    PaleGreen,
    PaleTurquoise,
    PaleVioletRed,
    PapayaWhip,
    PeachPuff,
    Peru,
    Pink,
    Plum,
    PowderBlue,
    RebeccaPurple,
    RosyBrown,
    RoyalBlue,
    Salmon,
    SandyBrown,
    SeaGreen,
    Seashell,
    Sienna,
    SkyBlue,
    Snow,
    SpringGreen,
    SteelBlue,
    Tan,
    Thistle,
    Tomato,
    Turquoise,
    Violet,
    Wheat,
    White,
    WhiteSmoke,
    Yellow,
    YellowGreen,
}
