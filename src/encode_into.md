Encodes directly into a buffer. Returns number of bytes written.
Buffer should have at least [`size`] bytes.

# Safety

Undefined Behaviour when the buffer's length is less than [`size`]`(bytes)`.