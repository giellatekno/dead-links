#!/usr/bin/env python

import json
import subprocess
import shlex
import pathlib
import tempfile
import os


def red(s):
    return f"\033[31m{s}\033[0m"


def green(s):
    return f"\033[32m{s}\033[0m"


def yellow(s):
    return f"\033[33m{s}\033[0m"


def get_tests():
    def no_expected_lines(obj):
        return [
            "didn't expect any lines on stdout, as this test i marked "
            "should-succeed"
        ]

    tests = []

    # first, the tests that shouldn't fail. (which have "should-succeed": True)

    # test: two-files-one-working-link
    tests.append({
        "name": "two-files-one-working-link",
        "should-succeed": True,
        "tree": {
            "index.md": "[working link](file.html)",
            "file.md": "a simple file",
        },
    })

    # test: relative-link-to-parent
    tests.append({
        "name": "relative-link-to-parent",
        "should-succeed": True,
        "tree": {
            "index.md": "empty or whatever",
            "subdir": {
                "somefile.md": "../index.html",
            },
        },
    })

    # test: relative-link-to-missing-parent
    tests.append({
        "name": "relative-link-to-missing-parent",
        "should-succeed": True,
        "tree": {
            "index.md": "empty or whatever",
            "subdir": {
                "somefile.md": "../missing.html",
            },
        },
    })

    # test: absolute-link-to-parent
    tests.append({
        "name": "absoute-link-to-parent",
        "should-succeed": True,
        "tree": {
            "index.md": "empty or whatever",
            "subdir": {
                "somefile.md": "/index.html",
            },
        },
    })

    # test: external-link
    tests.append({
        "name": "external-link",
        "should-succeed": True,
        "tree": {
            "index.md": "[google](https://google.com/)",
        },
    })

    # now the tests that should fail. check that the failure is what we expect

    # test: one-file-one-broken-link
    tests.append({
        "name": "one-file-one-broken-link",
        "tree": {
            "index.md": "[broken](nonexistant.html)"
        },
        "fn": "one_file_one_broken_link",
    })

    def one_file_one_broken_link(json_line):
        """
        Files: index.md, has one link to a nonexistant file.
        Checks that we get an error on this nonexistant file.
        """
        errors = []
        if json_line["type"] != "nonexistant":
            return errors

        # checks the link: does it exist? It shouldn't.
        exists = pathlib.Path(json_line["resolved_to"]).exists()
        if exists:
            errors.append("type=nonexistant: file exists when it shouldn't")
        return errors

    # test: links-outside-root
    tests.append({
        "name": "links-outside-root",
        "tree": {
            "index.md": "[outside root](../somefile.html)",
        },
        "fn": "links_outside_root",
    })

    def links_outside_root(json_line):
        expected = {
            "type": "outside-root",
            "file": "index.md",
            "lineno": 1,
            "url": "../somefile.html",
        }

        actual = json_line
        if expected != actual:
            return [(
                "json line error object was not as expected.\nexpected\n"
                f"{expected}\ngot:\n{actual}\n"
            )]
        return []

    # test: unintentional-md-link
    tests.append({
        "name": "unintentional-md-link",
        "tree": {
            "index.md": "[link text](page.md)",
        },
        "fn": "unintentional_md_link",
    })

    def unintentional_md_link(json_line):
        expected = {
            "type": "md",
            "file": "index.md",
            "lineno": 1,
            "url": "page.md",
        }
        actual = json_line

        # we don't know the full path of the temporary directory, but we don't really
        # need it either.
        del actual["resolved_to"]
        if expected != actual:
            return [(
                "json line error object was not as expected.\nexpected\n"
                f"{expected}\ngot:\n{actual}\n"
            )]
        return []

    # can't refer to a function before its defined, so in the dictionary for
    # each test, the 'fn' key has a string of the function name. here we just
    # replace that string, with the name from the locals() with that name, to
    # actually have the function itself under the 'fn' key, not just the name
    # of it.
    for test in tests:
        fn = test.get("fn")
        if fn is None:
            test["fn"] = no_expected_lines
        else:
            test["fn"] = locals()[test["fn"]]

    return tests


def make_tree(tree, path):
    for name, value in tree.items():
        if isinstance(value, str):
            with open(os.path.join(path, name), "w") as f:
                f.write(value)
        elif isinstance(value, dict):
            p = os.path.join(path, name)
            os.mkdir(p)
            make_tree(value, p)


def main():
    total, succeeded, failed, ignored = 0, 0, 0, 0
    for test in get_tests():
        total += 1
        name = test["name"]
        tree = test["tree"]
        fn = test["fn"]
        should_succeed = test.get("should-succeed", False)

        print(f"test '{name}' ... ", end="", flush=True)

        if test.get("ignored") is True:
            ignored += 1
            print(yellow("ignored"))
            continue

        path = tempfile.mkdtemp()
        make_tree(tree, path)

        command = f"cargo run --release -- {path} --json"
        proc = subprocess.run(
            shlex.split(command),
            capture_output=True,
            text=True,
        )

        if proc.returncode != 0:
            failed += 1
            print(red("failed"))
            print(f"command failed with non-zero exit code: {proc.returncode}")
            print(f"command: {command}")
            print("stderr:")
            print(f"{proc.stderr}")
            continue

        errors = []
        stdout_lines = proc.stdout.splitlines()

        if should_succeed:
            if stdout_lines:
                print(red("failed"))
                print(
                    "Error: Test should succeed, but didn't. There should have "
                    "been no errors reported on stdout from running, but there "
                    "was, and they are:")
                print(stdout_lines)
            else:
                # it should succeed, and there was no output lines, so we're good
                succeeded += 1
                print(green("ok"))
        else:
            if not stdout_lines:
                # test should not succeed, so it should have error lines, but there were
                # none!
                print(red("failed"))
                print("Error: [meta] expected error lines, but stdout had nothing")
            else:
                # test should not succeed, and it didn't, because there was error lines
                # so check that they are the expected errors

                for line in stdout_lines:
                    obj = json.loads(line)
                    errors.extend(fn(obj))

                if not errors:
                    succeeded += 1
                    print(green("ok"))
                else:
                    print(red("failed"))
                    for error in errors:
                        print(f"Error: {error}")

    assert total == succeeded + failed + ignored
    if not failed:
        result = green("ok")
    else:
        result = red("failed")

    print(f"\ntest result: {result}. {succeeded} passed; {failed} failed; {ignored} ignored")


if __name__ == "__main__":
    raise SystemExit(main())


# XXX unused: alternative test directory structure representation
# potentially clearer than the nested dicts one in use now?
T = [
    ("index.md", "some data"),
    ("subdir/file.md", "some data"),
]

