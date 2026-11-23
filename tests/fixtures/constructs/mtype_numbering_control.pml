/* Control: the same model asserting a = 1, which is what declaration-order numbering from 0 would give.
   pan refutes it, so the pair distinguishes the two numberings rather than merely checking one. */
mtype = { a, b, c };
mtype x;

active proctype P() { x = a; assert(x == 1) }
