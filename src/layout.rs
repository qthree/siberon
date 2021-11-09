use keyberon::action::{k, l, Action::*, /*HoldTapConfig, m*/};
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
                match self.layer[y+shift_y][x+shift_x] {
                    Trans => {
                        self.layer[y+shift_y][x+shift_x] = to_combine;
                    }
                    _ => {
                        loop {/*Attempt to overwrite already combined key*/}
                    },
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
        [k(Grave),     k(Kb1), k(Kb2),      k(Kb3),        k(Kb4), k(Kb5),],
        [k(Tab),       k(Q),   k(W),        k(E),          k(R),   k(T),],
        [k(BSpace),    k(A),   k(S),        k(D),          k(F),   k(G),],
        [k(Delete),    k(Z),   k(X),        k(C),          k(V),   k(B),],
    ]
}

#[rustfmt::skip]
const fn left_thumb() -> [[Action; 4]; 2] {
    [
        [k(LBracket), k(RBracket), k(Space), k(Enter),],
        [                                 k(LGui), k(LShift),
                                          k(LCtrl), k(LAlt),
        ]
    ]
}

#[rustfmt::skip]
const fn right_letters() -> [[Action; 6]; 4] {
    [
        [k(Kb6), k(Kb7), k(Kb8),      k(Kb9),    k(Kb0),    Trans,],
        [k(Y),   k(U),   k(I),        k(O),      k(P),      Trans,],
        [k(H),   k(J),   k(K),        k(L),      k(SColon), Trans,],
        [k(N),   k(M),   k(Comma),    k(Dot),    k(Slash),  Trans,],
    ]
}

#[rustfmt::skip]
const fn right_thumb() -> [[Action; 4]; 2] {
    [
        [k(Space), k(Enter), k(LBracket), k(RBracket),],
        [k(LGui), l(1),
         k(LCtrl), k(LAlt),
        ]
    ]
}

static MAIN: LayerCombiner = LayerCombiner::new()
    .combine(left_letters(), 0, 0)
    .combine(left_thumb(), 2, 4)
    .combine(right_letters(), 6, 0)
    .combine(right_thumb(), 6, 4);


const fn fn_keys() -> [[Action; 12]; 1] {
    [[k(F1), k(F2), k(F3), k(F4),  k(F5),  k(F6),
      k(F7), k(F8), k(F9), k(F10), k(F11), k(F12)]]
}

static FN: LayerCombiner = LayerCombiner::new()
    .combine(fn_keys(), 0, 0);

pub static LAYERS: Layers = layers!(
    MAIN, FN
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