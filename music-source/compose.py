#!/usr/bin/env python3
"""Author Meteor Sizzle; explicit regeneration replaces edited project/patch sources."""
import json
from pathlib import Path

SOURCE = Path(__file__).resolve().parent


def save(path, value):
    path.write_text(json.dumps(value, indent=2) + '\n')


def note(key, tick, duration=90, velocity=96):
    return dict(pitch=key, start_tick=tick, duration_ticks=duration, velocity=velocity)


def main():
    specifications = [
        ('grit_bass',0,'saw',0.27,0.15,True),
        ('orbital_riff',1,'square',0.19,0.16,True),
        ('lead_answer',2,'triangle',0.22,0.19,True),
        ('meteor_kick',0,'sine',0.80,0.14,False),
        ('meteor_snare',0,'noise',0.42,0.09,False),
        ('orbit_hat',0,'noise',0.14,0.025,False),
        ('dry_shot',47,'triangle',0.33,0.055,True),
        ('heavy_rock',58,'saw',0.87,0.19,True),
        ('middle_rock',57,'square',0.76,0.14,True),
        ('bright_rock',56,'triangle',0.83,0.12,True),
        ('ship_crash',55,'saw',0.90,0.24,True),
    ]
    instruments=[]
    for index,(name,program,waveform,gain,decay,pitched) in enumerate(specifications):
        patch=dict(name=name,sample_rate=32000,seed=11800+index,gain=gain,
                   duration_seconds=1.0,pitch_tracking=pitched,
                   oscillators=[dict(waveform=waveform,start_frequency_hz=440.0 if pitched else 175.0,
                                     end_frequency_hz=440.0 if pitched else 42.0,amplitude=1.0)],
                   envelope=dict(attack_seconds=0.001,decay_seconds=decay,sustain_level=0.0,release_seconds=0.05))
        # Octave voicing preserves shared cursor pitches while grading rock weight.
        frequency={'heavy_rock':220.0,'bright_rock':880.0}.get(name,440.0)
        if pitched:
            patch['oscillators'][0]['start_frequency_hz']=frequency
            patch['oscillators'][0]['end_frequency_hz']=frequency
        if name in ('grit_bass','heavy_rock','middle_rock','ship_crash','orbital_riff'):
            patch['filter']=dict(cutoff_hz={'grit_bass':1000.0,'heavy_rock':850.0,'middle_rock':1600.0,'ship_crash':1400.0,'orbital_riff':2400.0}[name],
                                 envelope_amount=1.4,resonance=0.22,stages=2)
        if name == 'grit_bass':
            patch['waveshaper']=dict(drive=2.5,contour=0.0,level=0.65)
        if name in ('bright_rock','lead_answer'):
            patch['oscillators'].append(dict(waveform='sine',start_frequency_hz=frequency*2,end_frequency_hz=frequency*2,amplitude=0.25))
        if name in ('meteor_snare','orbit_hat'):
            patch['filter']=dict(mode='high_pass',cutoff_hz=1400.0 if name=='meteor_snare' else 6600.0,stages=1)
        save(SOURCE/'patches'/f'{name}.json',patch)
        instruments.append(dict(name=name,source=dict(type='synth',patch=f'patches/{name}.json'),midi_program=program,gain=1.0,pan=0.0))
    tracks=[dict(name=name,instrument=instrument,notes=[]) for name,instrument in
            [('EngineRoom',0),('OrbitRiff',1),('CometAnswer',2),('Kick',3),('Snare',4),('Hat',5),
             ('theme',7),('drum',6),('surge',7),('crash',10),('fanfare',9),('life',9),('jam',9)]]
    # A three-plus-two rock phrase: clipped repeated fifths followed by a falling answer.
    hooks=[
        [(0,66,160),(360,73,100),(600,73,100),(960,69,240),(1440,76,160),(1800,73,100),(2160,66,240)],
        [(0,69,160),(360,66,100),(720,73,160),(1200,76,160),(1680,73,160),(2160,69,240)],
        [(0,73,160),(360,76,100),(600,73,100),(960,69,240),(1440,66,160),(1920,64,160)],
        [(0,66,360),(720,69,120),(1080,73,120),(1440,76,360),(2160,73,240)],
        [(0,78,160),(360,81,100),(600,85,100),(960,88,240),(1440,85,160),(1800,81,100),(2160,78,240)],
        [(0,85,160),(360,81,100),(720,78,160),(1200,76,160),(1680,73,160),(2160,76,240)],
        [(0,88,160),(360,85,100),(600,81,100),(960,78,240),(1440,76,160),(1920,73,160)],
        [(0,81,360),(720,78,120),(1080,76,120),(1440,73,360),(2160,66,240)],
    ]
    for bar in range(32):
        start=bar*2400
        reduced=16<=bar<24
        for offset,key in [(0,30),(360,42),(720,37),(960,30),(1440,40),(1800,37),(2160,33)]:
            if not reduced or offset in (0,1440):
                tracks[0]['notes'].append(note(key,start+offset,130,114 if offset in (0,1440) else 94))
        if not reduced or bar%2==0:
            for offset,key,length in hooks[bar%4+(4 if 8<=bar<16 else 0)]:
                tracks[1]['notes'].append(note(key,start+offset,length,94 if bar>=24 else 83))
        if bar%2==1:
            for offset,key in [(1560,85),(1920,81),(2160,78)]:
                tracks[2]['notes'].append(note(key,start+offset,150,87))
        for offset in ([0,1440] if reduced else [0,720,1440,1920]):
            tracks[3]['notes'].append(note(36,start+offset,90,118))
        for offset in [480,1680]:
            tracks[4]['notes'].append(note(38,start+offset,80,112))
        for offset in range(0,2400,480 if reduced else 240):
            tracks[5]['notes'].append(note(42,start+offset,35,77 if offset%480 else 48))
        if bar%8==7:
            for offset in [1920,2040,2160,2280]:
                tracks[4]['notes'].append(note(38,start+offset,65,88+(offset-1920)//15))
    tracks[0]['notes'].append(note(30,76680,120,114))
    for index,key in enumerate([66,73,69,76,73,66]*4):
        tracks[6]['notes'].append(note(key,index*240,130,118))
    # Continuous short gates preserve positional shots, including the loop seam.
    for tick in range(0,76800,120):
        tracks[7]['notes'].append(note([54,61,57,64,61][(tick//480)%5],tick,120,64))
    tracks[8]['notes'].append(note(42,0,65,48))
    for key in [42,49]:
        tracks[9]['notes'].append(note(key,0,400,122))
    for key in [78,81,85]:
        tracks[10]['notes'].append(note(key,0,400,122))
    tracks[11]['notes'].append(note(90,0,240,124))
    for index,key in enumerate([66,69,73,64]):
        tracks[12]['notes'].append(note(key,index*240,90,76))
    project=dict(format='midi-evil-project',version=2,name='Meteor Sizzle',tempo_bpm=160,
                 beats_per_bar=5,beat_unit=4,ticks_per_quarter=480,sample_rate=32000,
                 master_gain=0.8,instruments=instruments,tracks=tracks)
    (SOURCE/'meteor_sizzle.evil').write_text(json.dumps(project,separators=(',',':'))+'\n')
    presets,renders=[],[]
    for index,(name,program,_,_,_,pitched) in enumerate(specifications):
        keys={entry['pitch'] for track in tracks if track['instrument']==index for entry in track['notes']}
        if index in (8,9):
            keys.update(entry['pitch'] for entry in tracks[6]['notes'])
        zones=[]
        for key in sorted(keys):
            filename=f'rendered/{name}_{key}.wav'
            renders.append(dict(instrument=index,key=key,ticks=720,wav=filename))
            zones.append(dict(wav=filename,keys=[key,key],root=key,release_seconds=0.05,longest_note_seconds=0.42))
        if pitched:
            presets.append(dict(name=name,bank=0,program=program,zones=zones))
        else:
            percussion=next((preset for preset in presets if preset['bank']==128),None)
            if percussion is None:
                percussion=dict(name='Meteor drums',bank=128,program=0,zones=[])
                presets.append(percussion)
            percussion['zones'].extend(zones)
    save(SOURCE/'bank.json',dict(version=1,name='Meteor Sizzle',copyright='Original AI synthesis; Be Insiculous; CC0-1.0',presets=presets))
    save(SOURCE/'render.json',dict(seed=11800,sample_rate=32000,velocity=127,gain=0.8,
                                  backing_pitch_classes=[1,4,6,9],cue_pitch_classes=[1,4,6,9],renders=renders))


if __name__=='__main__':
    main()
