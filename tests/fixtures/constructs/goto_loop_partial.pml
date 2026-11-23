/* A claim that can only be violated by seeing x between the two assignments of a jump loop. */
byte x = 0;

active proctype A() {
L:	x = 1;
	x = 2;
	goto L
}

active proctype B() {
	assert(x != 1)
}
