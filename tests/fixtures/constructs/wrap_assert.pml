/* A byte written past its width. 250 + 10 is 260, which is 4 in a byte, so this asserts the truncation. */
byte i = 250;

active proctype P() { i = i + 10; assert(i == 4) }
