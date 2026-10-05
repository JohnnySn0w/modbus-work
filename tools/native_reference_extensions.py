"""Reviewed native device/reference additions beyond the Python baseline."""
import copy
import csv
import re

def extend(document, root):
    devices=document['devices']; regs=document['registers']
    devices['hmd65']['description']='Seven-entry Modbus Bridge configuration using one-based register numbers. Wet-bulb temperature, enthalpy, error code and measurement-status registers are excluded.'
    devices['bridge']['name']='Modbus Bridge'
    devices['bridge']['artifact']=None
    devices['synetica_usb']['name']='Modbus Bridge'
    devices['synetica_usb']['subtitle']='USB interface awaiting identification'
    devices['iaq_plus']['help_setup']=['TBD']; devices['iaq_plus']['help_troubleshooting']=['TBD']
    devices['hmd65']['help_setup']=['Suggested: confirm Modbus serial mode, slave address, baud rate and parity from the HMD65 configuration switches (Vaisala M212264EN-B).','Suggested: prefer the documented 32-bit floating-point register group; metric starts at 1 and non-metric starts at 129. Modbus Bridge tables use one-based register numbers. Direct Modbus addresses remain zero-based.']
    devices['hmd65']['help_troubleshooting']=['Check device and measurement status. Error-code registers 514-515 are excluded from Modbus Bridge tables.','Suggested: confirm the unit register group and word order against a known measurement before programming a Modbus Bridge profile.']
    devices['wattnode']['help_setup']=['Suggested: confirm the WND-M1-MB label and communication settings using WND-M1-MB-Ref-1.10.','Suggested: check current transformer primary ratings and electrical service mapping before interpreting current and power.']
    devices['wattnode']['help_troubleshooting']=['Suggested: compare native floating-point readings and status with the WND-M1-MB reference manual.','Suggested: preserve existing calibration values; do not treat CurrentIntScale as milliamps.']
    ati=copy.deepcopy(devices['hmd65']);ati.update(key='ati-f12',name='ATI F12',subtitle='Gas transmitter',kind='sensor',status='Ready to test',description='Eight-entry gas-transmitter configuration. Confirm concentration units and range for the installed sensor module.',artifact='artifacts/bridge-config/ati-badger-f12-d12-documentation-test.tsv',help_setup=['TBD'],help_troubleshooting=['TBD'])
    ati['readouts']=[{'label':x,'unit':u} for x,u in [('Gas concentration','sensor units'),('Temperature','°C'),('Loop output','mA')]];devices['ati-f12']=ati
    regs['ati-f12']=[]
    for row in csv.DictReader((root/'artifacts/register-tables/ati-badger-f12-d12-modbus-register-table.csv').open(encoding='utf-8-sig')):
        manual=[int(x) for x in row['register number'].split(' - ')];pdu=[x-40001 for x in manual]
        kind='F32 HL' if 'float32' in row['interpretation'] else ('S32 HL' if '32-bit' in row['interpretation'] else 'U16')
        name=row['name'];label={'Blanked concentration':'Gas concentration','Temperature':'Temperature','Loop output':'Loop output'}.get(name)
        unit={'deg C':'°C'}.get(row['units'],row['units']) or ('sensor units' if 'concentration' in name.lower() else '')
        regs['ati-f12'].append(dict(name=name,logical='–'.join(map(str,manual)),pdu='–'.join(map(str,pdu)),data_type=kind,access='R',unit=unit,description=(row['interpretation']+' '+row['decode']).strip(),readout_label=label,enum_values=[],bit_values=[(1 << int(bit), text.strip()) for bit, text in re.findall(r'bit (\d+) = ([^;]+)', row['decode']) if text.strip() != 'reserved'],zero_meaning='No flags set' if 'flags' in name.lower() else None))
    metric=[r for r in regs['hmd65'] if r['data_type'].startswith('F32')][:8]
    for i,r in enumerate(metric):
        r=copy.deepcopy(r);r['name']+=' (non-metric)';r['logical']='–'.join(str(int(x)+128) for x in r['logical'].split('–'));r['pdu']='–'.join(str(int(x)+128) for x in r['pdu'].split('–'));r['unit']=['%RH','°F','°F','°F','gr/ft³','gr/lb','°F','Btu/lb'][i];r['description']='Native non-metric 32-bit floating-point register group (Vaisala M212264EN-B).';regs['hmd65'].append(r)
    devices['wattnode']['description']='WND-M1-MB operation through Modbus Bridge is confirmed. Tested hardware and firmware versions are not recorded. Confirm current-transformer and service settings for the installation.'
    devices['ati-f12']['description']='F12 transmitter hardware 1.01 and software 1.25 confirmed working through Modbus Bridge. The tested gas module is not recorded; confirm units and range for the installed module.'
    from tools.native_reference_wattnode import extend_wattnode
    extend_wattnode(document, root)
    add_u1000(document, root)
    for device in devices.values():
        for field in ['name','subtitle','description']:
            device[field]=re.sub(r'\b(?:(?:E5|Synetica|Modbus)\s+)?bridge\b','Modbus Bridge',device[field],flags=re.I)
        for field in ['help_setup','help_troubleshooting']:
            device[field]=[re.sub(r'\b(?:(?:E5|Synetica|Modbus)\s+)?bridge\b','Modbus Bridge',s,flags=re.I).replace('COM3','the USB adapter') for s in device[field]]


def add_u1000(document, root):
    """Expose the documented heat-meter profile without assuming instrument units."""
    key = 'u1000mkii-hm'
    device = copy.deepcopy(document['devices']['hmd65'])
    device.update(key=key, name='Micronics U1000MKII-HM',
                  subtitle='Ultrasonic heat meter', kind='sensor',
                  status='Hardware verification pending',
                  description='Fifteen-entry Modbus Bridge profile from manufacturer documentation. Flow, heat and volume units follow the instrument settings.',
                  artifact='artifacts/bridge-config/micronics-u1000mkii-hm-documentation-test.tsv',
                  help_setup=['Suggested: confirm the slave address, serial settings and metric or imperial mode on the instrument (U1000MKII WM Issue 1.1, section 4.3).', 'Suggested: keep the instrument on its operating readout screen and allow at least one second between requests.'],
                  help_troubleshooting=['TBD'])
    device['readouts'] = [dict(label=name, unit=unit) for name, unit in
                          [('Measured velocity', 'm/s'), ('Measured flow', 'instrument units'),
                           ('Calculated power', 'instrument units')]]
    document['devices'][key] = device
    document['registers'][key] = []
    path = root / 'native/modbus-configurator/assets/u1000mkii-hm-registers.csv'
    with path.open(encoding='utf-8') as source:
        for row in csv.DictReader(source):
            addresses = [int(x) for x in row['register number'].split(' - ')]
            name = row['name']
            kind = 'F32 HH' if 'float32' in row['interpretation'] else ('S16' if name in ['Gain', 'Signal-to-noise ratio'] else 'U16')
            enums = {'System type': [(4, 'Heating'), (12, 'Chiller')],
                     'Instrument units': [(0, 'Metric'), (1, 'Imperial')]}.get(name, [])
            document['registers'][key].append(dict(
                name=name, logical='–'.join(str(x+40000) for x in addresses),
                pdu='–'.join(str(x-1) for x in addresses), data_type=kind,
                access='R', unit=row['units'],
                description=(row['interpretation']+' '+row['decode']).strip(),
                readout_label=name, enum_values=enums, bit_values=[],
                zero_meaning='No fault reported' if name == 'Status' else None))
