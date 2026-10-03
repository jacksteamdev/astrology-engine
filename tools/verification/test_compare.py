import copy
import importlib.util
import json
import tempfile
from pathlib import Path

spec = importlib.util.spec_from_file_location('comparator', Path(__file__).with_name('compare.py'))
comparator = importlib.util.module_from_spec(spec)
spec.loader.exec_module(comparator)


def test_negative_controls():
    expected = {'a': {'bits': ['3ff0000000000000']}, 'b': {'moment': None}}
    assert comparator.compare(expected, copy.deepcopy(expected))['passed']
    altered = copy.deepcopy(expected)
    altered['a']['bits'][0] = '3ff0000000000001'
    assert not comparator.compare(expected, altered)['passed']
    assert not comparator.compare(expected, {'a': expected['a']})['passed']
    assert not comparator.compare(expected, {**expected, 'c': {}})['passed']
    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory) / 'duplicate.jsonl'
        path.write_text('\n'.join(json.dumps({'id': 'a', 'value': value}) for value in expected.values()))
        try:
            comparator.load(path)
        except ValueError:
            pass
        else:
            raise AssertionError('Duplicate results passed validation')


if __name__ == '__main__':
    test_negative_controls()
    print('Comparator negative controls passed')
