/* The escape fires at x == 3, so the loop cannot reach 5. */
bool stop = false;
byte x = 0;

active proctype P() {
    do
    :: x < 5 -> x++
    od unless (x == 3)
}

ltl p0 { [] ( x <= 4 ) }
