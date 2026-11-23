/* The same claim with a do-loop instead of a label and goto. */
byte x = 0;

active proctype A() {
	do
	:: x = 1; x = 2
	od
}

active proctype B() {
	assert(x != 1)
}
