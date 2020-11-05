#!/usr/bin/env python3

import argparse
import sys
import json
from typing import Dict, List


def run():
    parser = argparse.ArgumentParser(description=f'Output digraph data for Cincinnati json',
                                     usage="curl -sH 'Accept:application/json' 'https://api.openshift.com/api/upgrades_info/v1/graph?channel=stable-4.5'  | ./graph.py --include-hotfixes | dot -Tsvg >graph.svg")
    parser.add_argument('--include-hotfixes', dest='hotfixes', action='store_true')
    parser.set_defaults(hotfixes=False)
    args = parser.parse_args()

    graph: Dict = json.load(sys.stdin)

    version_list: List[str] = list()  # a list of versions in the order returned by Cincy
    versions: Dict[str, Dict] = dict()  # maps version string to Cincy dict describing it
    edges: Dict[str, List] = dict()  # maps version string to list of version strings it has outgoing edges to

    for node in graph['nodes']:
        version = node['version']
        version_list.append(version)
        versions[version] = node
        # Ensure there is at least an empty list for all versions.
        edges[version] = []

    for edge_def in graph['edges']:
        # edge_def example [22, 20] where is number is an offset into versions
        from_ver = version_list[edge_def[0]]
        to_ver = version_list[edge_def[1]]
        edges[from_ver].append(to_ver)

    nodes_to_render = dict(versions)  # make a copy
    if not args.hotfixes:
        for version in versions.keys():
            if 'hotfix' in version or 'nightly' in version:
                nodes_to_render.pop(version)

    version_order = list(nodes_to_render.keys())

    print('digraph Upgrades {')
    print('  labelloc=t;')
    print('  rankdir=BT;')
    for index, version in enumerate(version_order):
        node = versions[version]
        url = node.get('metadata', {}).get('url', '')
        print(f'  {index} [ label="{version}" href="{url}" ];')

    for index, version in enumerate(version_order):
        for edge in edges[version]:
            if not args.hotfixes and ('hotfix' in edge or 'nightly' in edge):
                continue
            dest = version_order.index(edge)
            print(f'  {index}->{dest};')

    print('}')


if __name__ == '__main__':
    run()
