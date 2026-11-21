from typing import Optional


class Stack:
    """A bounded stack of strings."""

    def __init__(self, capacity: int = 8):
        if capacity < 1:
            raise ValueError("capacity must be at least 1")
        self._items: list[str] = []
        self._capacity = capacity

    def push(self, item: str) -> int:
        if len(self._items) >= self._capacity:
            raise OverflowError("stack is full")
        self._items.append(item)
        return len(self._items)

    def pop(self) -> Optional[str]:
        return self._items.pop() if self._items else None

    def peek(self) -> Optional[str]:
        return self._items[-1] if self._items else None

    def size(self) -> int:
        return len(self._items)
