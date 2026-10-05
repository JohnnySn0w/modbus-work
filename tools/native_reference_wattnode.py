"""Native WattNode integer bridge references, preserving legacy float definitions."""
import csv


def extend_wattnode(document, root):
    """Read selected wire addresses from the bridge profile and attach exact units."""
    registers = document['registers']['wattnode']
    source = root / 'artifacts/register-tables/wattnode-wnd-m1-mb-modbus-register-table.csv'
    with source.open(encoding='utf-8-sig') as stream:
        rows = {int(r['register number'].split(' - ')[0]) - 1: r for r in csv.DictReader(stream)}
    profile = root / 'artifacts/bridge-config/wattnode-wnd-m1-mb-documentation-test.tsv'
    with profile.open(encoding='utf-8-sig') as stream:
        for point in csv.DictReader(stream, delimiter='\t'):
            address = int(point['Addr'])
            row = rows[address]
            existing = next((r for r in registers if int(r['pdu'].split('–')[0]) == address), None)
            manual = [int(x) for x in row['register number'].split(' - ')]
            definition = dict(name=row['name'], logical='–'.join(map(str, manual)),
                                  pdu='–'.join(str(x-1) for x in manual),
                                  data_type=point['Data'] + (' ' + point['Word'] if len(manual) == 2 else ''),
                                  access='R', unit=row['units'], description=row['interpretation'],
                                  readout_label=row['name'], enum_values=[], bit_values=[], zero_meaning=None)
            if existing is None:
                registers.append(definition)
            else:
                # Preserve baseline positions, names and documented access for parity.
                for field in ['unit', 'description', 'data_type']:
                    existing[field] = definition[field]
    for register in registers:
        if 'W' in register['access'] and not register['name'].startswith('[CONFIG]'):
            register['name'] = '[CONFIG] ' + register['name']
    device = document['devices']['wattnode']
    device['description'] = 'Seventeen integer bridge entries. Net energy cannot be reset. Current requires the included scaling readbacks; [CONFIG] marks writable meter settings; the bridge reads them without changing them.'
    device['readouts'] = [dict(label='Lifetime net energy', unit='0.1 kWh'),
                          dict(label='Current 1', unit='current counts'),
                          dict(label='Voltage AB', unit='0.1 V')]
