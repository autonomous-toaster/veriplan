/* The same model without the clause, as the control: this one pan refutes. */
bool flag = false;

active proctype Hi() {
    do
    :: true -> skip
    od
}

active proctype Lo() {
    flag = true
}

ltl p0 { [] ( !flag ) }
