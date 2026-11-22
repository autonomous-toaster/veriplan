/* The same model with the escape checked only at the loop top - the wrong encoding written out by
   hand, so pan can be asked what it thinks of it. This file is not a construct the engine must
   support; it is the control that proves unless_midbody.pml discriminates the two encodings. */
byte x = 0;

active proctype P() {
    do
    :: x < 5 -> x = 1; x = x + 10
    :: x >= 1 -> break
    od
}

ltl p0 { [] ( x <= 1 ) }
