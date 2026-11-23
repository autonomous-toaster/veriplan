/* The same model with the same property unboxed. pan evaluates an unwrapped formula at the initial state,
   where a == 1 is false, so the implication is vacuously true and pan reports 0 - while the boxed version of
   the same words is 1. The two are not the same property, which is why the engine declines this one rather
   than guessing an encoding. */
byte a = 0;
byte b = 0;

active proctype P() { a = 1 }

ltl p0 { a == 1 -> <> ( b == 1 ) }
