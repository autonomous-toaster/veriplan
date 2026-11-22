/* The handler is armed by the FIRST statement of the body, so correct unless semantics must
   interleave between the two statements of that body - not at the top of the loop.

   x = 1 arms the escape; a correct encoding leaves the loop before x = x + 10 runs, so x is 1
   and the claim holds. An encoding that checks the escape only at the loop top lets the body
   finish, x becomes 11 and the same claim is refuted. The pair of files therefore discriminates
   the two encodings, which is what makes this model worth having. */
byte x = 0;

active proctype P() {
    do
    :: x < 5 -> x = 1; x = x + 10
    od unless (x == 1)
}

ltl p0 { [] ( x <= 1 ) }
