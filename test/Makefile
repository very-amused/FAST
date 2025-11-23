src-tests=test1-stream.c test2-stream-cb.c
bin-tests=$(src-tests:.c=)
libfast=../target/debug/libfast.a
override CFLAGS = -O1 -I../include/ -lgcc_s -lutil -lrt -lpthread -lm -ldl -lc -fsanitize=address

# Prefer building with clang
ifneq (,$(shell which clang))
	CC=clang
else
	CC=gcc
endif

define compile-test
$2: $1 $(libfast)
	clang $$(CFLAGS) -o$$@ $$^
endef

all: $(bin-tests)
.PHONY: all

# Generate test rules
$(foreach f,$(src-tests),$(eval $(call compile-test,$(f),$(f:.c=))))

clean:
	rm $(bin-tests)
.PHONY: clean
