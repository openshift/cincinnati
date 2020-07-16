use rgb::*;

fn main() {

    let px = RGB{r:255_u8,g:0,b:100};
    assert_eq!([px].as_bytes()[0], 255);

    let bigpx = RGB16{r:65535_u16,g:0,b:0};
    assert_eq!(bigpx.as_slice()[0], 65535);

    let px = RGB8::new(255, 0, 255);
    let inverted: RGB8 = px.map(|ch| 255 - ch);

    println!("{}", inverted); // rgb(0,255,0)
}
