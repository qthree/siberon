use keyberon::action::{k, l, m, Action::*, /*HoldTapConfig, m*/};
use keyberon::key_code::KeyCode::*;
use super::{ROWS, COLS};

pub type Action = keyberon::action::Action<()>;
pub type Layers = keyberon::layout::Layers<()>;

struct LayerCombiner {
    layer: [[Action; COLS]; ROWS],
}
impl LayerCombiner {
    const fn new() -> Self {
        LayerCombiner{
            layer: [[Trans; COLS]; ROWS],
        }
    }
    const fn combine<const R: usize, const C: usize>(mut self, to_combine: [[Action; C]; R], shift_x: usize, shift_y: usize) -> Self {
        let mut y = 0;
        while y<R {
            let mut x = 0;
            while x<C {
                let to_combine = to_combine[y][x];
                match to_combine {
                    Trans => {},
                    _ => {
                        match self.layer[y+shift_y][x+shift_x] {
                            Trans => {
                                self.layer[y+shift_y][x+shift_x] = to_combine;
                            }
                            _ => {
                                loop {/*Attempt to overwrite already combined key*/}
                            },
                        }
                    }
                }
                x+=1;
            }
            y+=1;
        }
        self
    }
}

macro_rules! layers {
    ($($combiner:ident),+) => {
        &[$(
            {
                static LAYER: &[[Action; COLS]; ROWS] = &$combiner.layer;
                static LAYER_ROWS: [&[Action]; ROWS] = [
                    &LAYER[0],
                    &LAYER[1],
                    &LAYER[2],
                    &LAYER[3],
                    &LAYER[4],
                    &LAYER[5],
                ];
                &LAYER_ROWS
            }
        ),+]
    };
}

#[rustfmt::skip]
const fn left_letters() -> [[Action; 6]; 4] {
    [
        [k(Grave),   k(Kb1), k(Kb2),      k(Kb3),        k(Kb4), k(Kb5),],
        [k(Tab),     k(Q),   k(W),        k(E),          k(R),   k(T),],
        [k(Escape),  k(A),   k(S),        k(D),          k(F),   k(G),],
        [k(Enter),   k(Z),   k(X),        k(C),          k(V),   k(B),],
    ]
}

#[rustfmt::skip]
const fn left_thumb() -> [[Action; 4]; 2] {
    match [
        [k(Minus),   k(Equal),     k(Space), k(LShift),],
        [                                 k(LGui), k(LCtrl),
                                          l(1),  k(LAlt),
        ]
    ]
    {
        [row0, [a, b, c, d]] => [
            row0,
            [c, d, a, b],
        ]
    }
}

#[rustfmt::skip]
const fn right_letters() -> [[Action; 6]; 4] {
    [
        [k(Kb6), k(Kb7), k(Kb8),      k(Kb9),    k(Kb0),    k(PScreen)],
        [k(Y),   k(U),   k(I),        k(O),      k(P),      Trans],
        [k(H),   k(J),   k(K),        k(L),      k(SColon), k(Quote),],
        [k(N),   k(M),   k(Comma),    k(Dot),    k(Slash),  k(Bslash),],
    ]
}

#[rustfmt::skip]
const fn right_thumb() -> [[Action; 4]; 2] {
    [
        [k(BSpace), k(Delete), k(LBracket), k(RBracket),],
        [m(&[RGui, Space]), Trans,
         Trans, l(1),
        ]
    ]
}

static MAIN: LayerCombiner = LayerCombiner::new()
    .combine(left_letters(), 0, 0)
    .combine(left_thumb(), 2, 4)
    .combine(right_letters(), 6, 0)
    .combine(right_thumb(), 6, 4);


const fn fn_keys() -> [[Action; 10]; 1] {
    [[k(F1), k(F2), k(F3), k(F4),  k(F5),
      k(F6), k(F7), k(F8), k(F9), k(F10),]]
}

const fn arrows() -> [[Action; 3]; 2] {
    [
        [Trans,      k(Up),   Trans,],
        [k(Left), k(Down), k(Right),],
    ]
}


const fn home_end() -> [[Action; 5]; 3] {
    [
        [k(ScrollLock),   Trans,      Trans,    Trans,        k(PgUp)],
        [k(Home),         Trans,      Trans,    Trans,       k(End)],
        [k(Insert),       Trans,      Trans,    Trans,        k(PgDown)],
    ]
}

const fn numpad() -> [[Action; 5]; 4] {
    [
        [Trans,   k(Kp7),      k(Kp8),    k(Kp9),        k(F11)],
        [Trans,   k(Kp4),      k(Kp5),    k(Kp6),       k(F12)],
        [Trans,   k(Kp1),      k(Kp2),    k(Kp3),        Trans],
        [Trans,   Trans,          k(Kp0),    k(KpDot),        k(KpEnter)],
    ]
}


static FIRST: LayerCombiner = LayerCombiner::new()
    .combine(fn_keys(), 1, 0)
    .combine(arrows(), 2, 1)
    .combine(home_end(), 1, 1)
    .combine(numpad(), 6, 1);

pub static LAYERS: Layers = layers!(
    MAIN, FIRST
);

#[cfg(test)]
mod test {
    extern crate std;
    use std::print;
    use keyberon::{debounce::Event, layout::Layout};

    use super::*;
    #[test]
    fn print_layers() {
        for layer in LAYERS {
            for row in *layer {
                for key in *row {
                    print!("{:?} ", key);
                }
                print!("\n");
            }
            print!("\n\n");
        }
    }
    #[test]
    fn test_layout() {
        let mut layout = Layout::new(LAYERS);
        layout.event(Event::Press(3, 8)); // y x
        layout.tick();
        assert_eq!(layout.keycodes().next(), Some(keyberon::key_code::KeyCode::Comma));
    }
}