#!/usr/bin/env python3

import argparse
import os
import random
import time


def chat_driver(
        word_source: str,
        delay_seconds: float,
        jitter_millis: int,
        phrase_length: int
):
    with open(word_source, 'r') as fp:
        words = fp.readlines()

    while True:
        phrase = ' '.join([
            x.strip()
            for x in random.choices(words, k=phrase_length)
        ])

        print(phrase, flush=True)

        sleep_time_seconds = delay_seconds + \
            (random.randrange(-1 * jitter_millis, jitter_millis) / 1000.0)

        time.sleep(sleep_time_seconds)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(
        description="A source of random phrases to pipe into chat "
        "clients' stdin"
    )

    parser.add_argument(
        '--word_source',
        '-w',
        help='where to get words from (default: %(default)s)',
        default=os.path.join(os.path.dirname(os.path.abspath(__file__)), 'web2a')
    )

    parser.add_argument(
        '--phrase_length',
        '-l',
        type=int,
        help='phrase length in words',
        default=5
    )

    parser.add_argument(
        '--delay_seconds',
        '-d',
        type=float,
        help='delay between phrases in seconds (default: %(default)s seconds)',
        default=2.0
    )

    parser.add_argument(
        '--jitter_millis',
        '-j',
        type=int,
        help='maximum time jitter between phrases in milliseconds '
        '(default: %(default)s milliseconds)',
        default=50
    )

    args = parser.parse_args()

    chat_driver(**vars(args))
