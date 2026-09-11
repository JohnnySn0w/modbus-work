"""Reviewed native device/reference additions beyond the Python baseline."""
import copy
import csv
import re

def extend(document, root):
    devices=document['devices']; regs=document['registers']
    devices['bridge']['name']='E5 bridge'
    devices['bridge']['artifact']=None
    devices['synetica_usb']['name']='E5 bridge'
    devices['synetica_usb']['subtitle']='USB interface awaiting identification'
    devices['iaq_plus']['help_setup']=['TBD']; devices['iaq_plus']['help_troubleshooting']=['TBD']
    devices['hmd65']['help_setup']=['Suggested: confirm Modbus serial mode, slave address, baud rate and parity from the HMD65 configuration switches (Vaisala M212264EN-B).','Suggested: prefer the documented 32-bit floating-point register group; metric starts at 1 and non-metric starts at 129. Addresses sent on the wire are one lower.']
    devices['hmd65']['help_troubleshooting']=['Suggested: check device and measurement status bitfields; error code occupies 514–515.','Suggested: confirm the unit register group and word order against a known measurement before programming an E5 bridge profile.']
    devices['wattnode']['help_setup']=['Suggested: confirm the WND-M1-MB label and communication settings using WND-M1-MB-Ref-1.10.','Suggested: check current transformer primary ratings and electrical service mapping before interpreting current and power.']
    devices['wattnode']['help_troubleshooting']=['Suggested: compare native floating-point readings and status with the WND-M1-MB reference manual.','Suggested: preserve existing calibration values; do not treat CurrentIntScale as milliamps.']
    ati=copy.deepcopy(devices['hmd65']);ati.update(key='ati-f12',name='ATI F12 — peracetic acid',subtitle='Toxic gas transmitter · peracetic acid sensor module',kind='sensor',status='Ready to test',description='Register configuration for the peracetic acid sensor module. Testing with a connected sensor is pending. Other sensor modules: TBD.',artifact='artifacts/bridge-config/ati-badger-f12-d12-documentation-test.tsv',help_setup=['TBD'],help_troubleshooting=['TBD'])
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
    for device in devices.values():
        for field in ['name','subtitle','description']:
            device[field]=re.sub(r'(?<!E5 )\bbridge\b','E5 bridge',device[field],flags=re.I)
        for field in ['help_setup','help_troubleshooting']:
            device[field]=[re.sub(r'(?<!E5 )\bbridge\b','E5 bridge',s,flags=re.I).replace('COM3','the USB adapter') for s in device[field]]
