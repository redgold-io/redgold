from typing import Annotated, List
from langchain.tools import tool


@tool
def multiply_by_max(
    a: Annotated[int, "scale factor"],
    b: Annotated[List[int], "list of ints over which to take maximum"],
) -> int:
    """Multiply a by the maximum of b."""
    return a * max(b)


# print(multiply_by_max.as_tool.to)
