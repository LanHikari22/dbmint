import os
from typing import Any, Callable, Dict, Generic, Iterable, Tuple, List, TypeVar, TypedDict, Union
import inspect
import checkpipe as pipe

from result import Err, Ok, Result

IO_T = TypeVar('IO_T')

# Signifies that a function performs an I/O effect. Only Functions
class IO(Generic[IO_T]):
    """
    Signifies that an IO Effect occured. Only functions that return IO Effects may extract this by
    reflection. 
    This is not a description of an effect-producing program but just a tag that an effect occured.
    """
    def __init__(self, result: IO_T):
        self._result = result
    
    def extract(self, caller_returns_IO: bool) -> IO_T:
        if not caller_returns_IO:
            raise Exception("Pure functions cannot extract effect")
        else:
            return self._result

    def reflect_extract(self) -> IO_T:
        # Check the return type of the caller via annotations
        caller_frame = inspect.stack()[1]  # Get the previous frame in the stack
        caller_function = caller_frame.function  # Get the name of the caller function
        caller_annotations = caller_frame.frame.f_globals.get(caller_function).__annotations__

        # Access the return type annotation
        return_type = caller_annotations.get('return', None)

        if return_type is not IO:
            raise Exception(f"Caller function '{caller_function}' must return an IO object. Expected: {IO}, Got: {return_type}.")
        else:
            return self._result


AppError_Enums = TypeVar('AppError_Enums')

class AppError(Generic[AppError_Enums]):
    def __init__(self, errno: AppError_Enums, details: str='', data: Dict[str, Any]={}):
        self.errno = errno
        self.details = details
        self.data = data
    
    def __str__(self) -> str:
        return str(self.__dict__)
    
    def __repr__(self) -> str:
        return str(self)


Common1_T = TypeVar('Common1_T')
def unwrap_or_print_and_exit_on_err(result: Result[Common1_T, str], exit_code: int=1) -> IO[Common1_T]:
    if result.is_ok():
        return IO(result.unwrap())
    else:
        print(result.unwrap_err())
        exit(exit_code)


def print_and_exit_on(cond: bool, s: str, exit_code: int=1) -> IO[None]:
    if cond:
        print(s)
        exit(exit_code)

    return IO(None)

def os_system(command: str) -> IO[None]:
    # print('$ ' + command)
    os.system(command)
    return IO(None)

