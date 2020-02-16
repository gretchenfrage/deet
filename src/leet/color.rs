
/// Macro for creating/printing ANSI-colored string literals 
/// and formatted strings.
macro_rules! color {
    (@fg(black))=>{"30"};
    (@fg(red))=>{"31"};
    (@fg(green))=>{"32"};
    (@fg(yellow))=>{"33"};
    (@fg(blue))=>{"34"};
    (@fg(purple))=>{"35"};
    (@fg(cyan))=>{"36"};
    (@fg(white))=>{"37"};
    (@reset)=>{"\x1B[0m"};

    // color part case
    (
        @fstr($accum:expr) $fg:ident $part:expr ; $($tail:tt)*
    )=>{
        color!(
            @fstr(concat!(
                $accum,
                "\x1B[",
                color!(@fg($fg)),
                "m",
                $part,
                color!(@reset)
            ))
            $($tail)*
        )
    };
    
    // uncolored part case
    (
        @fstr($accum:expr) $part:expr ; $($tail:tt)*
    )=>{
        color!(
            @fstr(concat!($accum, $part))
            $($tail)*
        )
    };
    
    // base case
    (
        @fstr($fstr:expr) , $($arg:tt)*
    )=>{
        println!($fstr, $($arg)*)
    };
    // base case
    (
        @fstr($fstr:expr) str
    )=>{ $fstr };
    // base case
    (
        @fstr($fstr:expr) format, $($arg:tt)*
    )=>{ format!($fstr, $($arg)*) };
    
    // bootstrapping
    ($($arg:tt)*)=>{ color!(@fstr("") $($arg)*) };
}