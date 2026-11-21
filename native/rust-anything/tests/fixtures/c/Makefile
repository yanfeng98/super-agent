CFLAGS ?= -O2 -Wall -Isrc

checksum: src/main.c src/checksum.c src/text.c
	$(CC) $(CFLAGS) -o $@ $^

test: tests/test_checksum.c src/checksum.c src/text.c
	$(CC) $(CFLAGS) -o test_checksum $^ && ./test_checksum
