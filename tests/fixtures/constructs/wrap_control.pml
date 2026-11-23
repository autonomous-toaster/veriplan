/* Control: the same model asserting the untruncated value. pan refutes it, so the pair distinguishes a write
   that wraps from one that does not rather than merely checking one. */
byte i = 250;

active proctype P() { i = i + 10; assert(i == 260) }
