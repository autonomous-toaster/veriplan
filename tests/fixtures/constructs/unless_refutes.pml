/* Same escape as unless.pml, but the claim is true only up to 2: the escape lets x reach 3,
   so pan refutes. Together with unless.pml this brackets what the escape means. */
bool stop = false;
byte x = 0;

active proctype P() {
    do
    :: x < 5 -> x++
    od unless (x == 3)
}

ltl p0 { [] ( x <= 2 ) }
