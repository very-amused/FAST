src-tests=test1-stream.c
bin-tests=$(src-tests:.c=)
libfast=../target/debug/libfast.a
override CFLAGS = -O1 -I../include/

define compile-test
$2: $1
	$$(CC) $$(CFLAGS) -o$$@ $$< $(libfast)
endef

all=$(src-tests)

# Generate test rules
$(foreach f,$(src-tests),$(eval $(call compile-test,$(f),$(f:.c=))))

clean: $(bin-tests)
	rm $<
.PHONY: clean
