use keyberon::action::{k, l, m, Action::*, HoldTapConfig};
use keyberon::key_code::KeyCode::*;

#[rustfmt::skip]
pub static LAYERS: keyberon::layout::Layers<()> = &[
    &[
        &[k(Grave),     k(Kb1), k(Kb2),      k(Kb3),        k(Kb4), k(Kb5),],
        &[k(Tab),       k(Q),   k(W),        k(E),          k(R),   k(T),],
        &[k(BSpace),    k(A),   k(S),        k(D),          k(F),   k(G),],
        &[k(Delete),    k(Z),   k(X),        k(C),          k(V),   k(B),],
        &[Trans,           Trans,      k(LBracket), k(RBracket),   Trans,     Trans],
        &[Trans,           Trans,      Trans,          Trans,            Trans,     Trans],
    ],
];
