
import socket
import sys

def check_server(host="localhost", port=1420):
    """
    Checks if a server is running on the given host and port.
    Returns True if reachable, False otherwise.
    """
    try:
        with socket.create_connection((host, port), timeout=1):
            return True
    except (OSError, ConnectionRefusedError):
        return False

if __name__ == "__main__":
    if check_server():
        print("Server is running.")
        sys.exit(0)
    else:
        print("Server is NOT running.")
        sys.exit(1)
