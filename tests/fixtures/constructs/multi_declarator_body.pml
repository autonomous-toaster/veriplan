/* Two variables of one type declared in one statement inside a proctype body. pan accepts it; the parser
   stops here.

   The same declaration at top level parses ("int k, i, j;" is a row that decides), and a single declarator
   inside a body parses ("pid k;" decides), so the gap is exactly the comma form in a body. It is worth a row
   because 24 of the 34 parse-error models over spin's shipped examples contain one of these - co-occurrence
   rather than proof, since splitting them does not by itself make those models parse. */
active proctype P() {
	byte o, i;
	o = 1;
	assert(o == 1)
}
