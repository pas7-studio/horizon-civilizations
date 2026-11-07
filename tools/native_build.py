#!/usr/bin/env python3
"""Explicit native Horizon mixin build gate on a machine with Rust + internet.

Never patches the user's original Horizon checkout. Source-only, no game
resources are included. An actual launch/E2E still requires owned MC assets.
"""
from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
COMPAT = json.loads((ROOT / 'compatibility.json').read_text())
NATIVE = ROOT / 'target' / 'native-check'


def run(*args: str, cwd: Path | None = None):
    print('>', ' '.join(args), flush=True)
    subprocess.run(list(args), cwd=cwd, check=True)


def checkout_repo(url: str, directory: Path, revision: str) -> None:
    if not (directory / '.git').exists():
        directory.parent.mkdir(parents=True, exist_ok=True)
        run('git', 'clone', '--filter=blob:none', url, str(directory))
    run('git', '-C', str(directory), 'fetch', 'origin', revision)
    run('git', '-C', str(directory), 'checkout', '--detach', revision)
    head = subprocess.check_output(['git', '-C', str(directory), 'rev-parse', 'HEAD'], text=True).strip()
    if head != revision:
        raise RuntimeError(f'expected {revision}, checked out {head}')
    if subprocess.check_output(['git', '-C', str(directory), 'status', '--porcelain'], text=True).strip():
        raise RuntimeError(f'dirty checkout at {directory}; refused')


def prepare_pristine_source(checkout: Path, staged: Path, revision: str) -> None:
    """Create a detached Git worktree with correct revision, no input symlinks.

    The mixin plan checks the worktree's HEAD against the manifest pin; a
    filesystem-only copy without .git would accidentally pick the *mod* repo's
    HEAD instead. We change only this disposable worktree, never checkout.
    """
    if staged.exists():
        shutil.rmtree(staged)
    run('git', '-C', str(checkout), 'worktree', 'prune')
    run('git', '-C', str(checkout), 'worktree', 'add', '--force', '--detach', str(staged), revision)
    observed = subprocess.check_output(
        ['git', '-C', str(staged), 'rev-parse', 'HEAD'], text=True,
    ).strip()
    if observed != revision:
        raise RuntimeError(f'native staging revision {observed} != {revision}')
    for link in list(staged.rglob('*')):
        if not link.is_symlink():
            continue
        target = link.resolve(strict=True)
        try:
            target.relative_to(staged.resolve())
        except ValueError as err:
            raise RuntimeError(f'external symlink refused: {link}') from err
        if not target.is_file():
            raise RuntimeError(f'non-file symlink refused: {link}')
        data = target.read_bytes()
        link.unlink()
        link.write_bytes(data)
    links = [str(path.relative_to(staged)) for path in staged.rglob('*') if path.is_symlink()]
    if links:
        raise RuntimeError(f'unresolved symlinks in native stage: {links}')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--test', action='store_true', help='run Horizon library tests after cargo check')
    parser.add_argument('--build-client', action='store_true', help='compile an actual Horizon client executable after checks (requires substantial RAM/disk/time)')
    args = parser.parse_args()
    for name in ('git', 'cargo'):
        if not shutil.which(name):
            raise SystemExit(f'{name} not installed. Install Rust 1.95 and Git first.')
    source = NATIVE / 'horizon'
    staged_source = NATIVE / 'horizon-stage'
    transformer = NATIVE / 'horizon-mixins'
    out = NATIVE / 'modded'
    plan = NATIVE / 'plan.json'
    checkout_repo(COMPAT['horizon_repository'], source, COMPAT['horizon_revision'])
    checkout_repo(COMPAT['horizon_mixins_repository'], transformer, COMPAT['horizon_mixins_revision'])
    prepare_pristine_source(source, staged_source, COMPAT['horizon_revision'])
    run('cargo', 'build', '--locked', '--manifest-path', str(transformer / 'Cargo.toml'),
        '-p', 'horizon-mixins-cli')
    binary = transformer / 'target/debug' / ('horizon-mixins.exe' if os.name == 'nt' else 'horizon-mixins')
    common = ['--workspace', str(staged_source), '--mods', str(ROOT / 'mixins')]
    run(str(binary), 'plan', *common, '--out', str(plan))
    if out.exists():
        shutil.rmtree(out)
    run(str(binary), 'apply', *common, '--out', str(out))
    run('cargo', 'check', '--manifest-path', str(out / 'Cargo.toml'),
        '--locked', '-p', 'horizon-client', '--no-default-features')
    if args.test:
        run('cargo', 'test', '--manifest-path', str(out / 'Cargo.toml'),
            '--locked', '-p', 'horizon-client', '--lib', '--no-default-features',
            'civilizations_core::tests')
    executable = None
    if args.build_client:
        # The 'fast' profile is documented by Horizon for iterating on native
        # client builds. No Mojang assets or login credentials are bundled.
        run('cargo', 'build', '--manifest-path', str(out / 'Cargo.toml'),
            '--locked', '-p', 'horizon-client', '--bin', 'horizon-client',
            '--profile', 'fast', '--no-default-features')
        executable = out / 'target' / 'fast' / ('horizon-client.exe' if os.name == 'nt' else 'horizon-client')
        if not executable.is_file():
            raise RuntimeError(f'cargo reported success, but executable is missing: {executable}')
    print(json.dumps({'mixin_plan': str(plan), 'modded_horizon': str(out),
                      'horizon_revision': COMPAT['horizon_revision'],
                      'cargo_check': 'PASS', 'native_client_binary': str(executable) if executable else 'NOT BUILT',
                      'gameplay_e2e': 'NOT TESTED'}, indent=2))


if __name__ == '__main__':
    main()
