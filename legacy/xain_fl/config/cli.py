"""This module provides helpers for parsing the CLI arguments.
"""
import argparse


def get_cmd_parameters() -> argparse.Namespace:
    """Parse the command arguments

    Returns:
        ~argparse.Namespace: the parsed command arguments
    """
    parser = argparse.ArgumentParser(description="Coordinator CLI")
    parser.add_argument(
        "--config",
        dest="config",
        default="xain-fl.toml",
        help="Path to the coordinator configuration file",
    )
    return parser.parse_args()
