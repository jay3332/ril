//! RGB color constants, which includes all preset colors specified in X11/CSS3.
//!
//! Only constants included in the list of X11 color names are included here. This is also known as
//! the CSS Color Module Level 1 color keywords.

use crate::pixel::Rgb;

// Pinks / Violets / Magentas

/// Represents the color `#c71585`, also known as `mediumvioletred`.
pub const MEDIUM_VIOLET_RED: Rgb = Rgb::new(199, 21, 133);

/// Represents the color `#ff1493`, also known as `deeppink`.
pub const DEEP_PINK: Rgb = Rgb::new(255, 20, 147);

/// Represents the color `#db7093`, also known as `palevioletred`.
pub const PALE_VIOLET_RED: Rgb = Rgb::new(219, 112, 147);

/// Represents the color `#ff69b4`, also known as `hotpink`.
pub const HOT_PINK: Rgb = Rgb::new(255, 105, 180);

/// Represents the color `#ffb6c1`, also known as `lightpink`.
pub const LIGHT_PINK: Rgb = Rgb::new(255, 182, 193);

/// Represents the color `#ffc0cb`, also known as `pink`.
pub const PINK: Rgb = Rgb::new(255, 192, 203);

/// Represents the color `#ff00ff`, also known as `magenta`.
pub const MAGENTA: Rgb = Rgb::new(255, 0, 255);

/// An alias for [`MAGENTA`], also known as `fuchsia`.
pub const FUCHSIA: Rgb = MAGENTA;

/// Represents the color `#ee82ee`, also known as `violet`.
pub const VIOLET: Rgb = Rgb::new(238, 130, 238);

/// Represents the color `#da70d6`, also known as `orchid`.
pub const ORCHID: Rgb = Rgb::new(218, 112, 214);

/// Represents the color `#dda0dd`, also known as `plum`.
pub const PLUM: Rgb = Rgb::new(221, 160, 221);

/// Represents the color `#d8bfd8`, also known as `thistle`.
pub const THISTLE: Rgb = Rgb::new(216, 191, 216);

/// Represents the color `#ba55d3`, also known as `mediumorchid`.
pub const MEDIUM_ORCHID: Rgb = Rgb::new(186, 85, 211);

/// Represents the color `#9932cc`, also known as `darkorchid`.
pub const DARK_ORCHID: Rgb = Rgb::new(153, 50, 204);

/// Represents the color `#9400d3`, also known as `darkviolet`.
pub const DARK_VIOLET: Rgb = Rgb::new(148, 0, 211);

/// Represents the color `#8b008b`, also known as `darkmagenta`.
pub const DARK_MAGENTA: Rgb = Rgb::new(139, 0, 139);

/// Represents the color `#8a2be2`, also known as `blueviolet`.
pub const BLUE_VIOLET: Rgb = Rgb::new(138, 43, 226);

/// Represents the color `#9370db`, also known as `mediumpurple`.
pub const MEDIUM_PURPLE: Rgb = Rgb::new(147, 112, 219);

/// Represents the color `#800080`, also known as `purple`.
pub const PURPLE: Rgb = Rgb::new(128, 0, 128);

/// Represents the color `#4b0082`, also known as `indigo`.
pub const INDIGO: Rgb = Rgb::new(75, 0, 130);

// Reds

/// Represents the color `#dc143c`, also known as `crimson`.
pub const CRIMSON: Rgb = Rgb::new(220, 20, 60);

/// Represents the color `#cd5c5c`, also known as `indianred`.
pub const INDIAN_RED: Rgb = Rgb::new(205, 92, 92);

/// Represents the color `#f08080`, also known as `lightcoral`.
pub const LIGHT_CORAL: Rgb = Rgb::new(240, 128, 128);

/// Represents the color `#fa8072`, also known as `salmon`.
pub const SALMON: Rgb = Rgb::new(250, 128, 114);

/// Represents the color `#e9967a`, also known as `darksalmon`.
pub const DARK_SALMON: Rgb = Rgb::new(233, 150, 122);

/// Represents the color `#ffa07a`, also known as `lightsalmon`.
pub const LIGHT_SALMON: Rgb = Rgb::new(255, 160, 122);

/// Represents the color `#b22222`, also known as `firebrick`.
pub const FIREBRICK: Rgb = Rgb::new(178, 34, 34);

/// Represents the color `#ff0000`, also known as `red`.
pub const RED: Rgb = Rgb::new(255, 0, 0);

/// Represents the color `#8b0000`, also known as `darkred`.
pub const DARK_RED: Rgb = Rgb::new(139, 0, 0);

// Oranges

/// Represents the color `#ff4500`, also known as `orangered`.
pub const ORANGE_RED: Rgb = Rgb::new(255, 69, 0);

/// Represents the color `#ff6347`, also known as `tomato`.
pub const TOMATO: Rgb = Rgb::new(255, 99, 71);

/// Represents the color `#ff7f50`, also known as `coral`.
pub const CORAL: Rgb = Rgb::new(255, 127, 80);

/// Represents the color `#ff8c00`, also known as `darkorange`.
pub const DARK_ORANGE: Rgb = Rgb::new(255, 140, 0);

/// Represents the color `#ffa500`, also known as `orange`.
pub const ORANGE: Rgb = Rgb::new(255, 165, 0);

// Yellows

/// Represents the color `#ffff00`, also known as `yellow`.
pub const YELLOW: Rgb = Rgb::new(255, 255, 0);

/// Represents the color `#ffffe0`, also known as `lightyellow`.
pub const LIGHT_YELLOW: Rgb = Rgb::new(255, 255, 224);

/// Represents the color `#fffacd`, also known as `lemonchiffon`.
pub const LEMON_CHIFFON: Rgb = Rgb::new(255, 250, 205);

/// Represents the color `#fafad2`, also known as `lightgoldenrodyellow`.
pub const LIGHT_GOLDENROD_YELLOW: Rgb = Rgb::new(250, 250, 210);

/// Represents the color `#ffefd5`, also known as `papayawhip`.
pub const PAPAYA_WHIP: Rgb = Rgb::new(255, 239, 213);

/// Represents the color `#ffe4b5`, also known as `moccasin`.
pub const MOCCASIN: Rgb = Rgb::new(255, 228, 181);

/// Represents the color `#ffdab9`, also known as `peachpuff`.
pub const PEACH_PUFF: Rgb = Rgb::new(255, 218, 185);

/// Represents the color `#eee8aa`, also known as `palegoldenrod`.
pub const PALE_GOLDENROD: Rgb = Rgb::new(238, 232, 170);

/// Represents the color `#f0e68c`, also known as `khaki`.
pub const KHAKI: Rgb = Rgb::new(240, 230, 140);

/// Represents the color `#bdb76b`, also known as `darkkhaki`.
pub const DARK_KHAKI: Rgb = Rgb::new(189, 183, 107);

/// Represents the color `#ffd700`, also known as `gold`.
pub const GOLD: Rgb = Rgb::new(255, 215, 0);

// Browns

/// Represents the color `#fff8dc`, also known as `cornsilk`.
pub const CORNSILK: Rgb = Rgb::new(255, 248, 220);

/// Represents the color `#ffebcd`, also known as `blanchedalmond`.
pub const BLANCHED_ALMOND: Rgb = Rgb::new(255, 235, 205);

/// Represents the color `#ffe4c4`, also known as `bisque`.
pub const BISQUE: Rgb = Rgb::new(255, 228, 196);

/// Represents the color `#ffdead`, also known as `navajowhite`.
pub const NAVAJO_WHITE: Rgb = Rgb::new(255, 222, 173);

/// Represents the color `#deb887`, also known as `burlywood`.
pub const BURLYWOOD: Rgb = Rgb::new(222, 184, 135);

/// Represents the color `#d2b48c`, also known as `tan`.
pub const TAN: Rgb = Rgb::new(210, 180, 140);

/// Represents the color `#bc8f8f`, also known as `rosybrown`.
pub const ROSY_BROWN: Rgb = Rgb::new(188, 143, 143);

/// Represents the color `#f4a460`, also known as `sandybrown`.
pub const SANDY_BROWN: Rgb = Rgb::new(244, 164, 96);

/// Represents the color `#daa520`, also known as `goldenrod`.
pub const GOLDENROD: Rgb = Rgb::new(218, 165, 32);

/// Represents the color `#b8860b`, also known as `darkgoldenrod`.
pub const DARK_GOLDENROD: Rgb = Rgb::new(184, 134, 11);

/// Represents the color `#cd853f`, also known as `peru`.
pub const PERU: Rgb = Rgb::new(205, 133, 63);

/// Represents the color `#d2691e`, also known as `chocolate`.
pub const CHOCOLATE: Rgb = Rgb::new(210, 105, 30);

/// Represents the color `#a0522d`, also known as `sienna`.
pub const SIENNA: Rgb = Rgb::new(160, 82, 45);

/// Represents the color `#8b4513`, also known as `saddlebrown`.
pub const SADDLE_BROWN: Rgb = Rgb::new(139, 69, 19);

/// Represents the color `#a52a2a`, also known as `brown`.
pub const BROWN: Rgb = Rgb::new(165, 42, 42);

/// Represents the color `#800000`, also known as `maroon`.
pub const MAROON: Rgb = Rgb::new(128, 0, 0);

// Greens

/// Represents the color `#adff2f`, also known as `greenyellow`.
pub const GREEN_YELLOW: Rgb = Rgb::new(173, 255, 47);

/// Represents the color `#7fff00`, also known as `chartreuse`.
pub const CHARTREUSE: Rgb = Rgb::new(127, 255, 0);

/// Represents the color `#7cfc00`, also known as `lawngreen`.
pub const LAWN_GREEN: Rgb = Rgb::new(124, 252, 0);

/// Represents the color `#00ff00`, also known as `lime`.
pub const LIME: Rgb = Rgb::new(0, 255, 0);

/// Represents the color `#32cd32`, also known as `limegreen`.
pub const LIME_GREEN: Rgb = Rgb::new(50, 205, 50);

/// Represents the color `#98fb98`, also known as `palegreen`.
pub const PALE_GREEN: Rgb = Rgb::new(152, 251, 152);

/// Represents the color `#90ee90`, also known as `lightgreen`.
pub const LIGHT_GREEN: Rgb = Rgb::new(144, 238, 144);

/// Represents the color `#00fa9a`, also known as `mediumspringgreen`.
pub const MEDIUM_SPRING_GREEN: Rgb = Rgb::new(0, 250, 154);

/// Represents the color `#00ff7f`, also known as `springgreen`.
pub const SPRING_GREEN: Rgb = Rgb::new(0, 255, 127);

/// Represents the color `#3cb371`, also known as `mediumseagreen`.
pub const MEDIUM_SEA_GREEN: Rgb = Rgb::new(60, 179, 113);

/// Represents the color `#2e8b57`, also known as `seagreen`.
pub const SEA_GREEN: Rgb = Rgb::new(46, 139, 87);

/// Represents the color `#228b22`, also known as `forestgreen`.
pub const FOREST_GREEN: Rgb = Rgb::new(34, 139, 34);

/// Represents the color `#008000`, also known as `green`.
pub const GREEN: Rgb = Rgb::new(0, 128, 0);

/// Represents the color `#006400`, also known as `darkgreen`.
pub const DARK_GREEN: Rgb = Rgb::new(0, 100, 0);

/// Represents the color `#9acd32`, also known as `yellowgreen`.
pub const YELLOW_GREEN: Rgb = Rgb::new(154, 205, 50);

/// Represents the color `#6b8e23`, also known as `olivedrab`.
pub const OLIVE_DRAB: Rgb = Rgb::new(107, 142, 35);

/// Represents the color `#808000`, also known as `olive`.
pub const OLIVE: Rgb = Rgb::new(128, 128, 0);

/// Represents the color `#556b2f`, also known as `darkolivegreen`.
pub const DARK_OLIVE_GREEN: Rgb = Rgb::new(85, 107, 47);

/// Represents the color `#8fbc8f`, also known as `darkseagreen`.
pub const DARK_SEA_GREEN: Rgb = Rgb::new(143, 188, 143);

// Cyans / Teals

/// Represents the color `#00ffff`, also known as `cyan`.
pub const CYAN: Rgb = Rgb::new(0, 255, 255);

/// An alias for [`CYAN`], also known as `aqua`.
pub const AQUA: Rgb = CYAN;

/// Represents the color `#e0ffff`, also known as `lightcyan`.
pub const LIGHT_CYAN: Rgb = Rgb::new(224, 255, 255);

/// Represents the color `#afeeee`, also known as `paleturquoise`.
pub const PALE_TURQUOISE: Rgb = Rgb::new(175, 238, 238);

/// Represents the color `#7fffd4`, also known as `aquamarine`.
pub const AQUAMARINE: Rgb = Rgb::new(127, 255, 212);

/// Represents the color `#40e0d0`, also known as `turquoise`.
pub const TURQUOISE: Rgb = Rgb::new(64, 224, 208);

/// Represents the color `#48d1cc`, also known as `mediumturquoise`.
pub const MEDIUM_TURQUOISE: Rgb = Rgb::new(72, 209, 204);

/// Represents the color `#00ced1`, also known as `darkturquoise`.
pub const DARK_TURQUOISE: Rgb = Rgb::new(0, 206, 209);

/// Represents the color `#5f9ea0`, also known as `cadetblue`.
pub const CADET_BLUE: Rgb = Rgb::new(95, 158, 160);

/// Represents the color `#20b2aa`, also known as `lightseagreen`.
pub const LIGHT_SEA_GREEN: Rgb = Rgb::new(32, 178, 170);

/// Represents the color `#008b8b`, also known as `darkcyan`.
pub const DARK_CYAN: Rgb = Rgb::new(0, 139, 139);

/// Represents the color `#008080`, also known as `teal`.
pub const TEAL: Rgb = Rgb::new(0, 128, 128);

// Blues

/// Represents the color `#b0e0e6`, also known as `powderblue`.
pub const POWDER_BLUE: Rgb = Rgb::new(176, 224, 230);

/// Represents the color `#add8e6`, also known as `lightblue`.
pub const LIGHT_BLUE: Rgb = Rgb::new(173, 216, 230);

/// Represents the color `#87cefa`, also known as `lightskyblue`.
pub const LIGHT_SKY_BLUE: Rgb = Rgb::new(135, 206, 250);

/// Represents the color `#87ceeb`, also known as `skyblue`.
pub const SKY_BLUE: Rgb = Rgb::new(135, 206, 235);

/// Represents the color `#00bfff`, also known as `deepskyblue`.
pub const DEEP_SKY_BLUE: Rgb = Rgb::new(0, 191, 255);

/// Represents the color `#1e90ff`, also known as `dodgerblue`.
pub const DODGER_BLUE: Rgb = Rgb::new(30, 144, 255);

/// Represents the color `#6495ed`, also known as `cornflowerblue`.
pub const CORNFLOWER_BLUE: Rgb = Rgb::new(100, 149, 237);

/// Represents the color `#b0c4de`, also known as `lightsteelblue`.
pub const LIGHT_STEEL_BLUE: Rgb = Rgb::new(176, 196, 222);

/// Represents the color `#4682b4`, also known as `steelblue`.
pub const STEEL_BLUE: Rgb = Rgb::new(70, 130, 180);

/// Represents the color `#4169e1`, also known as `royalblue`.
pub const ROYAL_BLUE: Rgb = Rgb::new(65, 105, 225);

/// Represents the color `#0000ff`, also known as `blue`.
pub const BLUE: Rgb = Rgb::new(0, 0, 255);

/// Represents the color `#0000cd`, also known as `mediumblue`.
pub const MEDIUM_BLUE: Rgb = Rgb::new(0, 0, 205);

/// Represents the color `#00008b`, also known as `darkblue`.
pub const DARK_BLUE: Rgb = Rgb::new(0, 0, 139);

/// Represents the color `#000080`, also known as `navy`.
pub const NAVY: Rgb = Rgb::new(0, 0, 128);

/// Represents the color `#191970`, also known as `midnightblue`.
pub const MIDNIGHT_BLUE: Rgb = Rgb::new(25, 25, 112);

/// Represents the color `#7b68ee`, also known as `mediumslateblue`.
pub const MEDIUM_SLATE_BLUE: Rgb = Rgb::new(123, 104, 238);

/// Represents the color `#6a5acd`, also known as `slateblue`.
pub const SLATE_BLUE: Rgb = Rgb::new(106, 90, 205);

/// Represents the color `#483d8b`, also known as `darkslateblue`.
pub const DARK_SLATE_BLUE: Rgb = Rgb::new(72, 61, 139);

// Whites

/// Represents the color `#ffffff`, also known as `white`.
pub const WHITE: Rgb = Rgb::new(255, 255, 255);

/// Represents the color `#fffafa`, also known as `snow`.
pub const SNOW: Rgb = Rgb::new(255, 250, 250);

/// Represents the color `#f0fff0`, also known as `honeydew`.
pub const HONEYDEW: Rgb = Rgb::new(240, 255, 240);

/// Represents the color `#f5fffa`, also known as `mintcream`.
pub const MINT_CREAM: Rgb = Rgb::new(245, 255, 250);

/// Represents the color `#f0ffff`, also known as `azure`.
pub const AZURE: Rgb = Rgb::new(240, 255, 255);

/// Represents the color `#f0f8ff`, also known as `aliceblue`.
pub const ALICE_BLUE: Rgb = Rgb::new(240, 248, 255);

/// Represents the color `#f8f8ff`, also known as `ghostwhite`.
pub const GHOST_WHITE: Rgb = Rgb::new(248, 248, 255);

/// Represents the color `#f5f5f5`, also known as `whitesmoke`.
pub const WHITE_SMOKE: Rgb = Rgb::new(245, 245, 245);

/// Represents the color `#fff5ee`, also known as `seashell`.
pub const SEASHELL: Rgb = Rgb::new(255, 245, 238);

/// Represents the color `#f5f5dc`, also known as `beige`.
pub const BEIGE: Rgb = Rgb::new(245, 245, 220);

/// Represents the color `#fdf5e6`, also known as `oldlace`.
pub const OLD_LACE: Rgb = Rgb::new(253, 245, 230);

/// Represents the color `#fffaf0`, also known as `floralwhite`.
pub const FLORAL_WHITE: Rgb = Rgb::new(255, 250, 240);

/// Represents the color `#fffff0`, also known as `ivory`.
pub const IVORY: Rgb = Rgb::new(255, 255, 240);

/// Represents the color `#faebd7`, also known as `antiquewhite`.
pub const ANTIQUE_WHITE: Rgb = Rgb::new(250, 235, 215);

/// Represents the color `#faf0e6`, also known as `linen`.
pub const LINEN: Rgb = Rgb::new(250, 240, 230);

/// Represents the color `#fff0f5`, also known as `lavenderblush`.
pub const LAVENDER_BLUSH: Rgb = Rgb::new(255, 240, 245);

/// Represents the color `#ffe4e1`, also known as `mistyrose`.
pub const MISTY_ROSE: Rgb = Rgb::new(255, 228, 225);

/// Represents the color `#e6e6fa`, also known as `lavender`.
pub const LAVENDER: Rgb = Rgb::new(230, 230, 250);

// Grays / Blacks

/// Represents the color `#000000`, also known as `black`.
pub const BLACK: Rgb = Rgb::new(0, 0, 0);

/// Represents the color `#696969`, also known as `dimgray`.
pub const DIM_GRAY: Rgb = Rgb::new(105, 105, 105);

/// An alias for [`DIM_GRAY`], also known as `dimgrey`.
pub const DIM_GREY: Rgb = DIM_GRAY;

/// Represents the color `#808080`, also known as `gray`.
pub const GRAY: Rgb = Rgb::new(128, 128, 128);

/// An alias for [`GRAY`], also known as `grey`.
pub const GREY: Rgb = GRAY;

/// Represents the color `#a9a9a9`, also known as `darkgray`.
pub const DARK_GRAY: Rgb = Rgb::new(169, 169, 169);

/// An alias for [`DARK_GRAY`], also known as `darkgrey`.
pub const DARK_GREY: Rgb = DARK_GRAY;

/// Represents the color `#d3d3d3`, also known as `lightgray`.
pub const LIGHT_GRAY: Rgb = Rgb::new(211, 211, 211);

/// An alias for [`LIGHT_GRAY`], also known as `lightgrey`.
pub const LIGHT_GREY: Rgb = LIGHT_GRAY;

/// Represents the color `#dcdcdc`, also known as `gainsboro`.
pub const GAINSBORO: Rgb = Rgb::new(220, 220, 220);

/// Represents the color `#708090`, also known as `slategray`.
pub const SLATE_GRAY: Rgb = Rgb::new(112, 128, 144);

/// An alias for [`SLATE_GRAY`], also known as `slategrey`.
pub const SLATE_GREY: Rgb = SLATE_GRAY;

/// Represents the color `#778899`, also known as `lightslategray`.
pub const LIGHT_SLATE_GRAY: Rgb = Rgb::new(119, 136, 153);

/// An alias for [`LIGHT_SLATE_GRAY`], also known as `lightslategrey`.
pub const LIGHT_SLATE_GREY: Rgb = LIGHT_SLATE_GRAY;

/// Represents the color `#2f4f4f`, also known as `darkslategray`.
pub const DARK_SLATE_GRAY: Rgb = Rgb::new(47, 79, 79);

/// An alias for [`DARK_SLATE_GRAY`], also known as `darkslategrey`.
pub const DARK_SLATE_GREY: Rgb = DARK_SLATE_GRAY;
