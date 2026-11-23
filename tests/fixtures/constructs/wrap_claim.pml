/* The same fact in a claim, which is how a plan would express it: a byte cannot exceed 255. */
byte i = 250;

active proctype P() { i = i + 10 }

ltl p0 { [] ( i <= 255 ) }
