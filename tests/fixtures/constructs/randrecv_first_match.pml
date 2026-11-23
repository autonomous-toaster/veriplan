/* A random receive where two messages match: [1, 2, 1] with c ?? 1. pan leaves [2, 1], so it searches
   forward from the head and takes the FIRST match; taking the last match leaves [1, 2] and the assert
   fails, so this file distinguishes "first" from "last" rather than merely "searches" from "head only".

   The two-receive probe elsewhere proves a search happens; this proves which element wins. */
chan c = [3] of { byte };
byte a = 0;
byte b = 0;

active proctype P() {
	c ! 1; c ! 2; c ! 1;
	c ?? 1;
	c ? a;
	c ? b;
	assert(a == 2)
}
