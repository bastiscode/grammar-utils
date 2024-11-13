def load_byte_vocab() -> list[list[int]]:
    """

    Returns a vocabulary containing all bytes as possible continuations.

    """
    return [[i] for i in range(256)]
