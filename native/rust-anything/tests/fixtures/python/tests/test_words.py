from textkit.words import slugify, chunk


def test_slugify():
    assert slugify("Héllo Wörld") == "hello-world"


def test_chunk():
    assert chunk([1, 2, 3], 2) == [[1, 2], [3]]
