/* The same claim with a do-loop instead of a label and goto.

   This file found a false green in spin-rs v0.5.38, measured here and confirmed by adding a claim
   that names x: with no claim reading x, the identity projection drops x, the two writes collapse,
   and the assertion is never reported (pan 1, v0.5.35 1, v0.5.38 0). With an `ltl` that names x the
   same engine reports 1, so the loss is the field, not the assertion machinery. A projection must
   keep whatever an assertion reads, not only the marker the assertion writes.
*/
byte x = 0;

active proctype A() {
	do
	:: x = 1; x = 2
	od
}

active proctype B() {
	assert(x != 1)
}
