import os
import ctypes
import ctypes.util
import shutil
import sys

def change_apparmor_profile(profile: str) -> int:
    # Load libapparmor dynamically
    lib_path = ctypes.util.find_library("apparmor")
    libaa = ctypes.CDLL(lib_path)

    aa_change_profile = libaa.aa_change_onexec
    aa_change_profile.argtypes = [ctypes.c_char_p]
    aa_change_profile.restype = ctypes.c_int

    c_profile = ctypes.create_string_buffer(profile.encode())
    result = aa_change_profile(c_profile)

    if result == -1:
        errno = ctypes.get_errno()
        raise Exception(f"Error ({errno}): {os.strerror(errno)}")

    return result

if __name__ == "__main__":
    with open("/proc/self/attr/current", "rt", encoding="utf-8") as attr:
        print("Running with profile", attr.read().strip())

    profile = os.environ.get("AA_PROFILE", "snap.snapcraft.snapcraft")
    result = change_apparmor_profile(profile)
    print("Running as", profile)
    os.execvp(sys.argv[1], [sys.argv[1]] + sys.argv[2:])
