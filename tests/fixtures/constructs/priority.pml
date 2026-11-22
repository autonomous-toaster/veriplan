/* pan runs only the highest-priority enabled process, so Lo never runs here:
   the claim holds. Without the priority clause the claim is refuted. */
bool flag = false;

active proctype Hi() priority 2 {
    do
    :: true -> skip
    od
}

active proctype Lo() {
    flag = true
}

ltl p0 { [] ( !flag ) }
