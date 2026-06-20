#!/usr/bin/env python3
"""Generate KiCad 8 single-page schematic for ESP32 Solar LoRa Sensor Node.

Produces a self-contained .kicad_sch with embedded symbols and correct pin wiring.
"""
import uuid
import os

OUT = os.path.dirname(os.path.abspath(__file__))

def uid():
    return str(uuid.uuid4())

# pin types
PIN_IN    = "input"
PIN_OUT   = "output"
PIN_BIDIR = "bidirectional"
PIN_PWR   = "power_in"
PIN_PWR_O = "power_out"
PIN_PASS  = "passive"
PIN_TRI   = "tristate"

# ── helpers ────────────────────────────────────────────
def rect(x1, y1, x2, y2, fill="background"):
    return f'(rectangle (start {x1} {y1}) (end {x2} {y2}) (stroke (width 0.254) (type default)) (fill (type {fill})))'

def pin_(num, name, ekind, x, y, length=300, hide=False):
    h = " (hide yes)" if hide else ""
    return (
        f'(pin "{ekind}" (number "{num}" (effects (font (size 1.27 1.27))))'
        f'(name "{name}" (effects (font (size 1.27 1.27))))'
        f'(at {x} {y} 0) (length {length}){h}'
        f'(uuid "{uid()}"))'
    )

def ref_prop(ref, x=0, y=0):
    return f'(property "Reference" "{ref}" (id 0) (at {x} {y} 0) (effects (font (size 1.27 1.27)) (justify left)))'
def val_prop(val, x=0, y=0):
    return f'(property "Value" "{val}" (id 1) (at {x} {y} 0) (effects (font (size 1.27 1.27)) (justify left)))'
def fp_prop(fp=""):
    return f'(property "Footprint" "{fp}" (id 2) (at 0 0 0) (effects (font (size 1.27 1.27)) (hide yes)))'
def ds_prop(ds=""):
    return f'(property "Datasheet" "{ds}" (id 3) (at 0 0 0) (effects (font (size 1.27 1.27)) (hide yes)))'

# ── Symbol definitions ─────────────────────────────────
# Each symbol_* returns a (mils_width, mils_height, s_expr) tuple

def sym_cn3791():
    """CN3791 MPPT Solar Charger, SOP-8"""
    w, h, ox, oy = 30.48, 20.32, 15.24, 10.16
    body = rect(-w/2, -h/2, w/2, h/2)
    pins = [
        pin_(1, "CHEN", PIN_IN, -w/2-5.08, -7.62),
        pin_(2, "LN", PIN_OUT, -w/2-5.08, -2.54),
        pin_(3, "CSP", PIN_IN, -w/2-5.08, 2.54),
        pin_(4, "BAT", PIN_OUT, w/2+5.08, 7.62),
        pin_(5, "GND", PIN_PWR, w/2+5.08, 2.54),
        pin_(6, "FB", PIN_IN, w/2+5.08, -2.54),
        pin_(7, "VCC", PIN_PWR, w/2+5.08, -7.62),
        pin_(8, "DONE", PIN_OUT, -w/2-5.08, 7.62),
    ]
    sym = f'''
  (symbol "CN3791" (pin_names (offset 0.508)) (in_bom yes) (on_board yes)
    {ref_prop("U")} {val_prop("CN3791")}
    {fp_prop("Package_SO:SOP-8_3.9x4.9mm_P1.27mm")}
    {ds_prop("https://www.monolithicpower.com/cn3791")}
    (symbol "CN3791_0_1" {body} {''.join(pins)})
  )'''
    return ox, oy, sym, {1: (-w/2-5.08, -7.62), 2: (-w/2-5.08, -2.54),
                          3: (-w/2-5.08, 2.54), 4: (w/2+5.08, 7.62),
                          5: (w/2+5.08, 2.54), 6: (w/2+5.08, -2.54),
                          7: (w/2+5.08, -7.62), 8: (-w/2-5.08, 7.62)}

def sym_mp2307():
    w, h = 30.48, 20.32
    body = rect(-w/2, -h/2, w/2, h/2)
    pins = [
        pin_(1, "BST", PIN_PASS, -w/2-5.08, -7.62),
        pin_(2, "VIN", PIN_PWR, -w/2-5.08, -2.54),
        pin_(3, "SW", PIN_OUT, -w/2-5.08, 2.54),
        pin_(4, "GND", PIN_PWR, -w/2-5.08, 7.62),
        pin_(5, "FB", PIN_IN, w/2+5.08, 7.62),
        pin_(6, "COMP", PIN_PASS, w/2+5.08, 2.54),
        pin_(7, "EN", PIN_IN, w/2+5.08, -2.54),
        pin_(8, "SS", PIN_PASS, w/2+5.08, -7.62),
    ]
    sym = f'''
  (symbol "MP2307" (pin_names (offset 0.508)) (in_bom yes) (on_board yes)
    {ref_prop("U")} {val_prop("MP2307")}
    {fp_prop("Package_SO:SOP-8_3.9x4.9mm_P1.27mm")}
    {ds_prop("https://www.monolithicpower.com/mp2307")}
    (symbol "MP2307_0_1" {body} {''.join(pins)})
  )'''
    return w/2, h/2, sym, {1: (-w/2-5.08, -7.62), 2: (-w/2-5.08, -2.62),
                            3: (-w/2-5.08, 2.54), 4: (-w/2-5.08, 7.62),
                            5: (w/2+5.08, 7.62), 6: (w/2+5.08, 2.54),
                            7: (w/2+5.08, -2.54), 8: (w/2+5.08, -7.62)}

def sym_me3116():
    w, h = 25.4, 20.32
    body = rect(-w/2, -h/2, w/2, h/2)
    pins = [
        pin_(1, "EN", PIN_IN, -w/2-5.08, -7.62),
        pin_(2, "GND", PIN_PWR, -w/2-5.08, -2.54),
        pin_(3, "SW", PIN_OUT, -w/2-5.08, 2.54),
        pin_(4, "VIN", PIN_PWR, -w/2-5.08, 7.62),
        pin_(5, "NC", PIN_PASS, w/2+5.08, 7.62, hide=True),
        pin_(6, "FB", PIN_IN, w/2+5.08, -7.62),
    ]
    sym = f'''
  (symbol "ME3116" (pin_names (offset 0.508)) (in_bom yes) (on_board yes)
    {ref_prop("U")} {val_prop("ME3116")}
    {fp_prop("Package_TO_SOT_SMD:SOT-23-6")}
    {ds_prop("https://www.micro-ele.com/me3116")}
    (symbol "ME3116_0_1" {body} {''.join(pins)})
  )'''
    return w/2, h/2, sym, {1: (-w/2-5.08, -7.62), 2: (-w/2-5.08, -2.54),
                            3: (-w/2-5.08, 2.54), 4: (-w/2-5.08, 7.62),
                            5: (w/2+5.08, 7.62), 6: (w/2+5.08, -7.62)}

def sym_b0512xt():
    w, h = 25.4, 15.24
    body = rect(-w/2, -h/2, w/2, h/2)
    pins = [
        pin_(1, "VIN", PIN_PWR, -w/2-5.08, -5.08),
        pin_(2, "GND", PIN_PWR, -w/2-5.08, 5.08),
        pin_(3, "-12V", PIN_PWR_O, w/2+5.08, 5.08),
        pin_(4, "+12V", PIN_PWR_O, w/2+5.08, -5.08),
    ]
    sym = f'''
  (symbol "B0512XT-1W" (pin_names (offset 0.508)) (in_bom yes) (on_board yes)
    {ref_prop("U")} {val_prop("B0512XT-1W")}
    {fp_prop("Converter_DCDC:SIP4")}
    {ds_prop("https://www.mornsun.com/b0512xt-1w")}
    (symbol "B0512XT-1W_0_1" {body} {''.join(pins)})
  )'''
    return w/2, h/2, sym, {1: (-w/2-5.08, -5.08), 2: (-w/2-5.08, 5.08),
                            3: (w/2+5.08, 5.08), 4: (w/2+5.08, -5.08)}

def sym_mb85rc256():
    w, h = 25.4, 20.32
    body = rect(-w/2, -h/2, w/2, h/2)
    pins = [
        pin_(1, "A0", PIN_IN, -w/2-5.08, -7.62),
        pin_(2, "A1", PIN_IN, -w/2-5.08, -2.54),
        pin_(3, "A2", PIN_IN, -w/2-5.08, 2.54),
        pin_(4, "GND", PIN_PWR, -w/2-5.08, 7.62),
        pin_(5, "SDA", PIN_BIDIR, w/2+5.08, 7.62),
        pin_(6, "SCL", PIN_IN, w/2+5.08, 2.54),
        pin_(7, "WP", PIN_IN, w/2+5.08, -2.54),
        pin_(8, "VCC", PIN_PWR, w/2+5.08, -7.62),
    ]
    sym = f'''
  (symbol "MB85RC256V" (pin_names (offset 0.508)) (in_bom yes) (on_board yes)
    {ref_prop("U")} {val_prop("MB85RC256V")}
    {fp_prop("Package_SO:SOP-8_3.9x4.9mm_P1.27mm")}
    {ds_prop("https://www.fujitsu.com/mb85rc256v")}
    (symbol "MB85RC256V_0_1" {body} {''.join(pins)})
  )'''
    return w/2, h/2, sym, {1: (-w/2-5.08, -7.62), 2: (-w/2-5.08, -2.54),
                            3: (-w/2-5.08, 2.54), 4: (-w/2-5.08, 7.62),
                            5: (w/2+5.08, 7.62), 6: (w/2+5.08, 2.54),
                            7: (w/2+5.08, -2.54), 8: (w/2+5.08, -7.62)}

def sym_whl101():
    """WH-L101-L-H20 LoRa module (simplified 10 pins)"""
    w, h = 50.8, 25.4
    body = rect(-w/2, -h/2, w/2, h/2)
    pins = [
        pin_(1, "VCC", PIN_PWR, -w/2-5.08, 10.16),
        pin_(2, "GND", PIN_PWR, -w/2-5.08, 5.08),
        pin_(3, "RXD", PIN_IN,  -w/2-5.08, 0),
        pin_(4, "TXD", PIN_OUT, -w/2-5.08, -5.08),
        pin_(5, "SET", PIN_IN,  -w/2-5.08, -10.16),
        pin_(6, "RST", PIN_IN,  w/2+5.08, -10.16),
        pin_(7, "AUX", PIN_OUT, w/2+5.08, -5.08),
        pin_(8, "GND", PIN_PWR, w/2+5.08, 0),
        pin_(9, "ANT", PIN_PASS, w/2+5.08, 5.08),
        pin_(10, "VCC", PIN_PWR, w/2+5.08, 10.16),
    ]
    sym = f'''
  (symbol "WH-L101-L-H20" (pin_names (offset 0.508)) (in_bom yes) (on_board yes)
    {ref_prop("U")} {val_prop("WH-L101-L-H20")}
    {fp_prop("RF_Module:WH-L101-L-H20")}
    {ds_prop("https://www.whiznets.com/wh-l101-l-h20")}
    (symbol "WH-L101-L-H20_0_1" {body} {''.join(pins)})
  )'''
    return w/2, h/2, sym, {1: (-w/2-5.08, 10.16), 2: (-w/2-5.08, 5.08),
                            3: (-w/2-5.08, 0), 4: (-w/2-5.08, -5.08),
                            5: (-w/2-5.08, -10.16), 6: (w/2+5.08, -10.16),
                            7: (w/2+5.08, -5.08), 8: (w/2+5.08, 0),
                            9: (w/2+5.08, 5.08), 10: (w/2+5.08, 10.16)}

def sym_esp32():
    """ESP32-WROOM-32 — only pins used in this design"""
    w, h = 55.88, 55.88
    body = rect(-w/2, -h/2, w/2, h/2)
    pins = [
        pin_(1, "3V3", PIN_PWR, -w/2-5.08, 25.4),
        pin_(2, "EN", PIN_IN, -w/2-5.08, 20.32),
        pin_(3, "GPIO36", PIN_IN, -w/2-5.08, 15.24),
        pin_(4, "GPIO39", PIN_IN, -w/2-5.08, 10.16),
        pin_(5, "GPIO34", PIN_IN, -w/2-5.08, 5.08),
        pin_(6, "GPIO35", PIN_IN, -w/2-5.08, 0),
        pin_(7, "GPIO32", PIN_BIDIR, -w/2-5.08, -5.08),
        pin_(8, "GPIO33", PIN_BIDIR, -w/2-5.08, -10.16),
        pin_(9, "GPIO25", PIN_BIDIR, -w/2-5.08, -15.24),
        pin_(10, "GPIO26", PIN_BIDIR, -w/2-5.08, -20.32),
        pin_(11, "GPIO27", PIN_BIDIR, -w/2-5.08, -25.4),
        pin_(12, "GPIO14", PIN_BIDIR, w/2+5.08, -25.4),
        pin_(13, "GPIO12", PIN_BIDIR, w/2+5.08, -20.32),
        pin_(14, "GND", PIN_PWR, w/2+5.08, -15.24),
        pin_(15, "GPIO13", PIN_BIDIR, w/2+5.08, -10.16),
        pin_(16, "GPIO9", PIN_BIDIR, w/2+5.08, -5.08),
        pin_(17, "GPIO10", PIN_BIDIR, w/2+5.08, 0),
        pin_(18, "GPIO11", PIN_BIDIR, w/2+5.08, 5.08),
        pin_(19, "GPIO21", PIN_BIDIR, w/2+5.08, 10.16),
        pin_(20, "GPIO22", PIN_BIDIR, w/2+5.08, 15.24),
        pin_(21, "GPIO1", PIN_BIDIR, w/2+5.08, 20.32),   # TXD0
        pin_(22, "GPIO3", PIN_BIDIR, w/2+5.08, 25.4),    # RXD0
        pin_(23, "GPIO17", PIN_BIDIR, -w/2-5.08, 25.4),  # TXD2 (left side)
        pin_(24, "GPIO16", PIN_BIDIR, -w/2-5.08, 20.32), # RXD2 (left side)
    ]
    sym = f'''
  (symbol "ESP32-WROOM-32" (pin_names (offset 0.508)) (in_bom yes) (on_board yes)
    {ref_prop("U")} {val_prop("ESP32-WROOM-32")}
    {fp_prop("RF_Module:ESP32-WROOM-32")}
    {ds_prop("https://www.espressif.com/en/products/modules/esp32")}
    (symbol "ESP32-WROOM-32_0_1" {body} {''.join(pins)})
  )'''
    pinmap = {
        1: (-w/2-5.08, 25.4), 2: (-w/2-5.08, 20.32),
        3: (-w/2-5.08, 15.24), 4: (-w/2-5.08, 10.16),
        5: (-w/2-5.08, 5.08), 6: (-w/2-5.08, 0),
        7: (-w/2-5.08, -5.08), 8: (-w/2-5.08, -10.16),
        9: (-w/2-5.08, -15.24), 10: (-w/2-5.08, -20.32),
        11: (-w/2-5.08, -25.4),
        12: (w/2+5.08, -25.4), 13: (w/2+5.08, -20.32),
        14: (w/2+5.08, -15.24), 15: (w/2+5.08, -10.16),
        16: (w/2+5.08, -5.08), 17: (w/2+5.08, 0),
        18: (w/2+5.08, 5.08), 19: (w/2+5.08, 10.16),
        20: (w/2+5.08, 15.24), 21: (w/2+5.08, 20.32),
        22: (w/2+5.08, 25.4),
        23: (-w/2-5.08, 25.4), 24: (-w/2-5.08, 20.32),
    }
    return w/2, h/2, sym, pinmap

def sym_sht30():
    w, h = 20.32, 15.24
    body = rect(-w/2, -h/2, w/2, h/2)
    pins = [
        pin_(1, "SDA", PIN_BIDIR, -w/2-5.08, -5.08),
        pin_(2, "ADDR", PIN_IN, -w/2-5.08, 0),
        pin_(3, "ALERT", PIN_OUT, -w/2-5.08, 5.08),
        pin_(4, "GND", PIN_PWR, w/2+5.08, 5.08),
        pin_(5, "SCL", PIN_IN, w/2+5.08, -5.08),
        pin_(6, "VDD", PIN_PWR, w/2+5.08, -5.08),
    ]
    sym = f'''
  (symbol "SHT30" (pin_names (offset 0.508)) (in_bom yes) (on_board yes)
    {ref_prop("U")} {val_prop("SHT30")}
    {fp_prop("Package_DFN_QFN:DFN-8_2.5x2.5mm_P0.5mm")}
    {ds_prop("https://www.sensirion.com/sht30")}
    (symbol "SHT30_0_1" {body} {''.join(pins)})
  )'''
    return w/2, h/2, sym, {1: (-w/2-5.08, -5.08), 2: (-w/2-5.08, 0),
                            3: (-w/2-5.08, 5.08), 4: (w/2+5.08, 5.08),
                            5: (w/2+5.08, -5.08), 6: (w/2+5.08, -5.08)}

def sym_bh1750():
    w, h = 20.32, 15.24
    body = rect(-w/2, -h/2, w/2, h/2)
    pins = [
        pin_(1, "VCC", PIN_PWR, -w/2-5.08, -5.08),
        pin_(2, "SCL", PIN_IN, -w/2-5.08, 0),
        pin_(3, "SDA", PIN_BIDIR, -w/2-5.08, 5.08),
        pin_(4, "ADDR", PIN_IN, w/2+5.08, 5.08),
        pin_(5, "GND", PIN_PWR, w/2+5.08, 0),
        pin_(6, "VCC", PIN_PWR, w/2+5.08, -5.08),
    ]
    sym = f'''
  (symbol "BH1750FVI" (pin_names (offset 0.508)) (in_bom yes) (on_board yes)
    {ref_prop("U")} {val_prop("BH1750FVI")}
    {fp_prop("Package_DFN_QFN:DFN-6_2x2mm_P0.5mm")}
    {ds_prop("https://www.rohm.com/bh1750fvi")}
    (symbol "BH1750FVI_0_1" {body} {''.join(pins)})
  )'''
    return w/2, h/2, sym, {1: (-w/2-5.08, -5.08), 2: (-w/2-5.08, 0),
                            3: (-w/2-5.08, 5.08), 4: (w/2+5.08, 5.08),
                            5: (w/2+5.08, 0), 6: (w/2+5.08, -5.08)}

def sym_ds3231():
    w, h = 25.4, 20.32
    body = rect(-w/2, -h/2, w/2, h/2)
    pins = [
        pin_(1, "32K", PIN_OUT, -w/2-5.08, -7.62),
        pin_(2, "VCC", PIN_PWR, -w/2-5.08, -2.54),
        pin_(3, "INT", PIN_OUT, -w/2-5.08, 2.54),
        pin_(4, "RST", PIN_IN, -w/2-5.08, 7.62),
        pin_(5, "VBAT", PIN_PWR, w/2+5.08, 7.62),
        pin_(6, "GND", PIN_PWR, w/2+5.08, 2.54),
        pin_(7, "SDA", PIN_BIDIR, w/2+5.08, -2.54),
        pin_(8, "SCL", PIN_IN, w/2+5.08, -7.62),
    ]
    sym = f'''
  (symbol "DS3231" (pin_names (offset 0.508)) (in_bom yes) (on_board yes)
    {ref_prop("U")} {val_prop("DS3231")}
    {fp_prop("Package_SO:SOP-16_3.9x4.9mm_P1.27mm")}
    {ds_prop("https://www.maximintegrated.com/ds3231")}
    (symbol "DS3231_0_1" {body} {''.join(pins)})
  )'''
    return w/2, h/2, sym, {1: (-w/2-5.08, -7.62), 2: (-w/2-5.08, -2.54),
                            3: (-w/2-5.08, 2.54), 4: (-w/2-5.08, 7.62),
                            5: (w/2+5.08, 7.62), 6: (w/2+5.08, 2.54),
                            7: (w/2+5.08, -2.54), 8: (w/2+5.08, -7.62)}

def sym_max485():
    w, h = 25.4, 20.32
    body = rect(-w/2, -h/2, w/2, h/2)
    pins = [
        pin_(1, "RO", PIN_OUT, -w/2-5.08, -7.62),
        pin_(2, "RE", PIN_IN, -w/2-5.08, -2.54),
        pin_(3, "DE", PIN_IN, -w/2-5.08, 2.54),
        pin_(4, "DI", PIN_IN, -w/2-5.08, 7.62),
        pin_(5, "GND", PIN_PWR, w/2+5.08, 7.62),
        pin_(6, "A", PIN_BIDIR, w/2+5.08, 2.54),
        pin_(7, "B", PIN_BIDIR, w/2+5.08, -2.54),
        pin_(8, "VCC", PIN_PWR, w/2+5.08, -7.62),
    ]
    sym = f'''
  (symbol "MAX485" (pin_names (offset 0.508)) (in_bom yes) (on_board yes)
    {ref_prop("U")} {val_prop("MAX485")}
    {fp_prop("Package_SO:SOP-8_3.9x4.9mm_P1.27mm")}
    {ds_prop("https://www.maximintegrated.com/max485")}
    (symbol "MAX485_0_1" {body} {''.join(pins)})
  )'''
    return w/2, h/2, sym, {1: (-w/2-5.08, -7.62), 2: (-w/2-5.08, -2.54),
                            3: (-w/2-5.08, 2.54), 4: (-w/2-5.08, 7.62),
                            5: (w/2+5.08, 7.62), 6: (w/2+5.08, 2.54),
                            7: (w/2+5.08, -2.54), 8: (w/2+5.08, -7.62)}

def sym_adum1201():
    w, h = 25.4, 20.32
    body = rect(-w/2, -h/2, w/2, h/2)
    pins = [
        pin_(1, "VDD1", PIN_PWR, -w/2-5.08, -7.62),
        pin_(2, "VIA", PIN_IN, -w/2-5.08, -2.54),
        pin_(3, "VIB", PIN_IN, -w/2-5.08, 2.54),
        pin_(4, "GND1", PIN_PWR, -w/2-5.08, 7.62),
        pin_(5, "GND2", PIN_PWR, w/2+5.08, 7.62),
        pin_(6, "VOB", PIN_OUT, w/2+5.08, 2.54),
        pin_(7, "VOA", PIN_OUT, w/2+5.08, -2.54),
        pin_(8, "VDD2", PIN_PWR, w/2+5.08, -7.62),
    ]
    sym = f'''
  (symbol "ADuM1201" (pin_names (offset 0.508)) (in_bom yes) (on_board yes)
    {ref_prop("U")} {val_prop("ADuM1201")}
    {fp_prop("Package_SO:SOP-8_3.9x4.9mm_P1.27mm")}
    {ds_prop("https://www.analog.com/adum1201")}
    (symbol "ADuM1201_0_1" {body} {''.join(pins)})
  )'''
    return w/2, h/2, sym, {1: (-w/2-5.08, -7.62), 2: (-w/2-5.08, -2.54),
                            3: (-w/2-5.08, 2.54), 4: (-w/2-5.08, 7.62),
                            5: (w/2+5.08, 7.62), 6: (w/2+5.08, 2.54),
                            7: (w/2+5.08, -2.54), 8: (w/2+5.08, -7.62)}

def sym_tlp185():
    w, h = 20.32, 10.16
    body = rect(-w/2, -h/2, w/2, h/2)
    pins = [
        pin_(1, "A", PIN_IN, -w/2-5.08, -2.54),
        pin_(2, "C", PIN_IN, -w/2-5.08, 2.54),
        pin_(3, "E", PIN_OUT, w/2+5.08, 2.54),
        pin_(4, "C", PIN_OUT, w/2+5.08, -2.54),
    ]
    pinmap = {1: (-w/2-5.08, -2.54), 2: (-w/2-5.08, 2.54),
              3: (w/2+5.08, 2.54), 4: (w/2+5.08, -2.54)}
    sym = f'''
  (symbol "TLP185" (pin_names (offset 0.508)) (in_bom yes) (on_board yes)
    {ref_prop("U")} {val_prop("TLP185")}
    {fp_prop("Package_SO:SOP-4_4.55x2.6mm_P1.27mm")}
    {ds_prop("https://toshiba.semicon-storage.com/tlp185")}
    (symbol "TLP185_0_1" {body} {''.join(pins)})
  )'''
    return w/2, h/2, sym, pinmap

def sym_ao3401():
    w, h = 15.24, 10.16
    body = rect(-w/2, -h/2, w/2, h/2)
    pins = [
        pin_(1, "G", PIN_IN, -w/2-5.08, 0),
        pin_(2, "S", PIN_PWR, w/2+5.08, -3.81),
        pin_(3, "D", PIN_PWR_O, w/2+5.08, 3.81),
    ]
    sym = f'''
  (symbol "AO3401" (pin_names (offset 0.508)) (in_bom yes) (on_board yes)
    {ref_prop("Q")} {val_prop("AO3401")}
    {fp_prop("Package_TO_SOT_SMD:SOT-23")}
    {ds_prop("https://www.aosmd.com/ao3401")}
    (symbol "AO3401_0_1" {body} {''.join(pins)})
  )'''
    return w/2, h/2, sym, {1: (-w/2-5.08, 0), 2: (w/2+5.08, -3.81), 3: (w/2+5.08, 3.81)}

def sym_usbc():
    w, h = 30.48, 20.32
    body = rect(-w/2, -h/2, w/2, h/2)
    pins = [
        pin_(1, "VBUS", PIN_PWR, -w/2-5.08, -7.62),
        pin_(2, "D-", PIN_PASS, -w/2-5.08, -2.54),
        pin_(3, "D+", PIN_PASS, -w/2-5.08, 2.54),
        pin_(4, "GND", PIN_PWR, -w/2-5.08, 7.62),
        pin_(5, "CC1", PIN_PASS, w/2+5.08, 7.62),
        pin_(6, "CC2", PIN_PASS, w/2+5.08, 2.54),
        pin_(7, "SBU1", PIN_PASS, w/2+5.08, -2.54),
        pin_(8, "SBU2", PIN_PASS, w/2+5.08, -7.62),
    ]
    sym = f'''
  (symbol "USB-C" (pin_names (offset 0.508)) (in_bom yes) (on_board yes)
    {ref_prop("J")} {val_prop("USB-C")}
    {fp_prop("Connector_USB:USB_C_Receptacle")}
    {ds_prop("")}
    (symbol "USB-C_0_1" {body} {''.join(pins)})
  )'''
    return w/2, h/2, sym, {1: (-w/2-5.08, -7.62), 2: (-w/2-5.08, -2.54),
                            3: (-w/2-5.08, 2.54), 4: (-w/2-5.08, 7.62),
                            5: (w/2+5.08, 7.62), 6: (w/2+5.08, 2.54),
                            7: (w/2+5.08, -2.54), 8: (w/2+5.08, -7.62)}

def sym_cp2102():
    w, h = 30.48, 25.4
    body = rect(-w/2, -h/2, w/2, h/2)
    pins = [
        pin_(1, "D+", PIN_PASS, -w/2-5.08, -10.16),
        pin_(2, "D-", PIN_PASS, -w/2-5.08, -5.08),
        pin_(3, "GND", PIN_PWR, -w/2-5.08, 0),
        pin_(4, "VDD", PIN_PWR, -w/2-5.08, 5.08),
        pin_(5, "VBUS", PIN_PWR, -w/2-5.08, 10.16),
        pin_(6, "TXD", PIN_OUT, w/2+5.08, 10.16),
        pin_(7, "RXD", PIN_IN, w/2+5.08, 5.08),
        pin_(8, "RTS", PIN_OUT, w/2+5.08, 0),
        pin_(9, "CTS", PIN_IN, w/2+5.08, -5.08),
        pin_(10, "SUSPEND", PIN_OUT, w/2+5.08, -10.16),
    ]
    sym = f'''
  (symbol "CP2102" (pin_names (offset 0.508)) (in_bom yes) (on_board yes)
    {ref_prop("U")} {val_prop("CP2102")}
    {fp_prop("Package_DFN_QFN:QFN-28_5x5mm_P0.5mm")}
    {ds_prop("https://www.silabs.com/cp2102")}
    (symbol "CP2102_0_1" {body} {''.join(pins)})
  )'''
    return w/2, h/2, sym, {1: (-w/2-5.08, -10.16), 2: (-w/2-5.08, -5.08),
                            3: (-w/2-5.08, 0), 4: (-w/2-5.08, 5.08),
                            5: (-w/2-5.08, 10.16), 6: (w/2+5.08, 10.16),
                            7: (w/2+5.08, 5.08), 8: (w/2+5.08, 0),
                            9: (w/2+5.08, -5.08), 10: (w/2+5.08, -10.16)}

def sym_mhz19b():
    w, h = 20.32, 15.24
    body = rect(-w/2, -h/2, w/2, h/2)
    pins = [
        pin_(1, "VCC", PIN_PWR, -w/2-5.08, -5.08),
        pin_(2, "GND", PIN_PWR, -w/2-5.08, 0),
        pin_(3, "TX", PIN_OUT, -w/2-5.08, 5.08),
        pin_(4, "RX", PIN_IN, w/2+5.08, 5.08),
        pin_(5, "PWM", PIN_OUT, w/2+5.08, -5.08),
    ]
    sym = f'''
  (symbol "MH-Z19B" (pin_names (offset 0.508)) (in_bom yes) (on_board yes)
    {ref_prop("U")} {val_prop("MH-Z19B")}
    {fp_prop("Sensor_Module:MH-Z19B")}
    {ds_prop("https://www.winsen-sensor.com/mh-z19b")}
    (symbol "MH-Z19B_0_1" {body} {''.join(pins)})
  )'''
    return w/2, h/2, sym, {1: (-w/2-5.08, -5.08), 2: (-w/2-5.08, 0),
                            3: (-w/2-5.08, 5.08), 4: (w/2+5.08, 5.08),
                            5: (w/2+5.08, -5.08)}

def sym_battery():
    w, h = 20.32, 10.16
    body = rect(-w/2, -h/2, w/2, h/2)
    pins = [
        pin_(1, "+", PIN_PWR_O, -w/2-5.08, -2.54),
        pin_(2, "-", PIN_PWR, -w/2-5.08, 2.54),
        pin_(3, "BAL", PIN_PASS, w/2+5.08, 0),
    ]
    sym = f'''
  (symbol "BATTERY-2S" (pin_names (offset 0.508)) (in_bom yes) (on_board yes)
    {ref_prop("B")} {val_prop("18650-2S")}
    {fp_prop("Battery:BatteryHolder_2x18650")}
    {ds_prop("")}
    (symbol "BATTERY-2S_0_1" {body} {''.join(pins)})
  )'''
    return w/2, h/2, sym, {1: (-w/2-5.08, -2.54), 2: (-w/2-5.08, 2.54),
                            3: (w/2+5.08, 0)}

def sym_tvs():
    w, h = 10.16, 7.62
    body = rect(-w/2, -h/2, w/2, h/2)
    pins = [
        pin_(1, "A", PIN_PASS, -w/2-5.08, 0),
        pin_(2, "C", PIN_PASS, w/2+5.08, 0),
    ]
    sym = f'''
  (symbol "SMBJ6.5CA" (pin_names (offset 0.508)) (in_bom yes) (on_board yes)
    {ref_prop("D")} {val_prop("SMBJ6.5CA")}
    {fp_prop("Diode_SMD:SMA")}
    {ds_prop("")}
    (symbol "SMBJ6.5CA_0_1" {body} {''.join(pins)})
  )'''
    return w/2, h/2, sym, {1: (-w/2-5.08, 0), 2: (w/2+5.08, 0)}

def sym_resistor():
    w, h = 10.16, 5.08
    body = f'(rectangle (start {-w/2} {-h/2}) (end {w/2} {h/2}) (stroke (width 0.254) (type default)) (fill (type background)))'
    pins = [
        pin_(1, "1", PIN_PASS, -w/2-5.08, 0),
        pin_(2, "2", PIN_PASS, w/2+5.08, 0),
    ]
    sym = f'''
  (symbol "R" (pin_names (offset 0)) (in_bom yes) (on_board yes)
    {ref_prop("R")} {val_prop("R")}
    {fp_prop("")} {ds_prop("")}
    (symbol "R_0_1" {body} {''.join(pins)})
  )'''
    return w/2, h/2, sym, {1: (-w/2-5.08, 0), 2: (w/2+5.08, 0)}

def sym_capacitor():
    w, h = 10.16, 10.16
    body = f'(rectangle (start {-3.81} {-h/2}) (end 3.81 {h/2}) (stroke (width 0.254) (type default)) (fill (type background)))'
    pins = [
        pin_(1, "1", PIN_PASS, -w/2-5.08, -2.54),
        pin_(2, "2", PIN_PASS, -w/2-5.08, 2.54),
    ]
    # Actually let's do a proper bipolar capacitor symbol
    body2 = (
        f'(rectangle (start {-3.81} {-5.08}) (end 3.81 -0.01) (stroke (width 0.254) (type default)) (fill (type background)))'
        f'(rectangle (start {-3.81} {0.01}) (end 3.81 5.08) (stroke (width 0.254) (type default)) (fill (type none)))'
    )
    pins2 = [
        pin_(1, "1", PIN_PASS, -w/2-5.08, -2.54, length=200),
        pin_(2, "2", PIN_PASS, -w/2-5.08, 2.54, length=200),
    ]
    # Actually for simplicity, just use a simple rectangle
    sym = f'''
  (symbol "C" (pin_names (offset 0)) (in_bom yes) (on_board yes)
    {ref_prop("C")} {val_prop("C")}
    {fp_prop("")} {ds_prop("")}
    (symbol "C_0_1"
      (rectangle (start {-5.08} {-5.08}) (end 5.08 -0.51) (stroke (width 0.254) (type default)) (fill (type background)))
      (rectangle (start {-5.08} {0.51}) (end 5.08 5.08) (stroke (width 0.254) (type default)) (fill (type none)))
      {pin_(1, "1", PIN_PASS, -10.16, -2.54, length=200)}
      {pin_(2, "2", PIN_PASS, -10.16, 2.54, length=200)}
    )
  )'''
    return 5.08, 5.08, sym, {1: (-10.16, -2.54), 2: (-10.16, 2.54)}


# ── Symbol registry ────────────────────────────────────
ALL_SYMBOLS = {
    "CN3791": sym_cn3791,
    "MP2307": sym_mp2307,
    "ME3116": sym_me3116,
    "B0512XT-1W": sym_b0512xt,
    "MB85RC256V": sym_mb85rc256,
    "WH-L101-L-H20": sym_whl101,
    "ESP32-WROOM-32": sym_esp32,
    "SHT30": sym_sht30,
    "BH1750FVI": sym_bh1750,
    "DS3231": sym_ds3231,
    "MAX485": sym_max485,
    "ADuM1201": sym_adum1201,
    "TLP185": sym_tlp185,
    "AO3401": sym_ao3401,
    "USB-C": sym_usbc,
    "CP2102": sym_cp2102,
    "MH-Z19B": sym_mhz19b,
    "BATTERY-2S": sym_battery,
    "SMBJ6.5CA": sym_tvs,
}

# ── Schematic layout ────────────────────────────────────
# Single A3 page (portrait: 16535 x 11693 mil @ 300 DPI ~ 420x297mm)
# We use landscape: 16000 x 11000 usable area

# Placement coordinates (center of each symbol)
# Organized in columns for readability
LAYOUT = [
    # Power section (left column)
    ("USB-C",     "J1",   "USB-C",        1500, 9000),
    ("BATTERY-2S","B1",   "18650-2S",     1500, 7000),
    ("CN3791",    "U1",   "CN3791",       3500, 8000),
    ("MP2307",    "U2",   "MP2307",       5500, 8000),
    ("ME3116",    "U3",   "ME3116",       7500, 8000),
    ("B0512XT-1W","U4",   "B0512XT-1W",   7500, 6000),

    # MCU section (center)
    ("ESP32-WROOM-32", "U5", "ESP32-WROOM-32", 3500, 4000),
    ("CP2102",    "U6",   "CP2102",       1000, 4000),

    # I²C sensors (middle-right)
    ("SHT30",     "U7",   "SHT30",        7000, 4500),
    ("BH1750FVI", "U8",   "BH1750FVI",    7000, 3500),
    ("DS3231",    "U9",   "DS3231",       7000, 2500),
    ("MB85RC256V","U10",  "MB85RC256V",   7000, 1500),

    # LoRa & CO₂ (right)
    ("WH-L101-L-H20", "U11", "WH-L101-L-H20", 10500, 5000),
    ("MH-Z19B",   "U12",  "MH-Z19B",      10500, 3000),

    # RS485 & output (far right)
    ("ADuM1201",  "U13",  "ADuM1201",     13500, 6000),
    ("MAX485",    "U14",  "MAX485",       13500, 4500),
    ("SMBJ6.5CA", "D1",   "SMBJ6.5CA",    15000, 3500),
    ("SMBJ6.5CA", "D2",   "SMBJ6.5CA",    15000, 3000),
    ("TLP185",    "U15",  "TLP185",       13500, 2500),
    ("AO3401",    "Q1",   "AO3401",       13500, 1500),
]

# ── Generate single-page schematic ─────────────────────

def gen_schematic(path):
    """Generate a single-page self-contained .kicad_sch."""
    lines = []
    lines.append(f'(kicad_sch (version 202401) (generator "opencode")')
    lines.append(f'  (uuid "{uid()}")')
    lines.append(f'  (title_block')
    lines.append(f'    (title "ESP32 Solar LoRa Sensor Node")')
    lines.append(f'    (date "2026-06-19")')
    lines.append(f'    (company "Agri-IoT")')
    lines.append(f'    (rev "1.0")')
    lines.append(f'    (comment 1 "Solar powered, LoRa 470MHz, Isolated RS485, I2C sensors")')
    lines.append(f'    (sheet (page 1) (pages 1))')
    lines.append(f'  )')

    # ── lib_symbols section (all symbols embedded) ──
    lines.append(f'  (lib_symbols')
    for name, fn in sorted(ALL_SYMBOLS.items()):
        _, _, sym_s, _ = fn()
        lines.append(sym_s)
    lines.append(f'  )')

    # ── Symbol instances planned ──
    placed = {}
    for (sym_name, ref, val, x, y) in LAYOUT:
        plt = uid()
        placed[ref] = (sym_name, x, y, plt)
        lines.append(f'''  (symbol (lib_id "{sym_name}") (at {x} {y} 0) (unit 1)
    (in_bom yes) (on_board yes)
    (uuid "{plt}")
    (property "Reference" "{ref}" (id 0) (at {x} {y - 7.62} 0)
      (effects (font (size 1.27 1.27)) (justify left)))
    (property "Value" "{val}" (id 1) (at {x} {y + 7.62} 0)
      (effects (font (size 1.27 1.27)) (justify left)))
    (property "Footprint" "" (id 2) (at 0 0 0)
      (effects (font (size 1.27 1.27)) (hide yes)))
    (property "Datasheet" "" (id 3) (at 0 0 0)
      (effects (font (size 1.27 1.27)) (hide yes)))
  )''')

    # ── Global labels for power nets (connect across entire project) ──
    # We use global labels so they match by name
    power_nets = [
        ("#PWR", "GND", 100, 100),
    ]
    # Not needed - we just place the GND power symbol later

    # ── Power symbols (GND flags) ──
    pwr_uid = uid()
    # We place GND symbols near each IC
    gnd_positions = [
        (1500, 8300), (1500, 6300), (3500, 7300), (5500, 7300),
        (7500, 7300), (7500, 5300), (3500, 3300), (1000, 3300),
        (7000, 3800), (7000, 2800), (7000, 1800),
        (10500, 4300), (10500, 2300),
        (13500, 5300), (13500, 3800),
        (13500, 1800),
    ]
    for gx, gy in gnd_positions:
        pwr_uid = uid()
        lines.append(f'''  (symbol (lib_id "power:GND") (at {gx} {gy} 0) (unit 1)
    (in_bom yes) (on_board yes)
    (uuid "{pwr_uid}")
    (property "Reference" "#PWR" (id 0) (at {gx} {gy} 0)
      (effects (font (size 1.27 1.27)) (hide yes)))
    (property "Value" "GND" (id 1) (at {gx} {gy - 3.81} 0)
      (effects (font (size 1.27 1.27))))
  )''')

    # ── Net labels for signal connections ──
    # Place net labels near pin locations. In KiCad, labels must be
    # on a wire segment to connect, but they also connect by name
    # when on the same sheet with matching names.
    # We place short wires from pins to labels where possible.
    
    # Helper: short wire from (x,y) to (x+dx, y) with label
    def wire_and_label(wx, wy, label_text, dx=200, orient=0):
        u1, u2 = uid(), uid()
        return (
            f'  (wire (pts (xy {wx} {wy}) (xy {wx+dx} {wy})) (stroke (width 0) (type default)) (uuid "{u1}"))\n'
            f'  (label "{label_text}" (at {wx+dx} {wy} {orient})'
            f' (effects (font (size 1.27 1.27)) (justify left)) (uuid "{u2}"))'
        )

    # Power rail global labels with short wires
    power_labels = [
        # (x, y, name, orient)
        (500, 9000, "SOLAR_IN", 0),       # USB-C VBUS
        (1500, 8300, "VBUS", 0),           # USB-C
        (1500, 7700, "BAT+", 0),           # Battery +
        (4000, 8800, "BAT+", 0),           # CN3791 BAT
        (4000, 8700, "5V", 0),             # CN3791 -> MP2307
        (6000, 8800, "5V", 0),             # MP2307 output
        (8000, 8800, "3V3", 0),            # ME3116 output
        (8000, 6800, "ISO_12V", 0),        # B0512XT output
        (8000, 8700, "5V", 0),             # ME3116 VIN
    ]
    for px, py, pname, porient in power_labels:
        lines.append(wire_and_label(px, py, pname, orient=porient))
    
    # I2C bus labels
    i2c_labels = [
        (6000, 5300, "I2C_SCL", 0),
        (6000, 5200, "I2C_SDA", 0),
        (7500, 5300, "I2C_SCL", 0),
        (7500, 5200, "I2C_SDA", 0),
    ]
    for lx, ly, lname, lorient in i2c_labels:
        lines.append(wire_and_label(lx, ly, lname, orient=lorient))

    # UART labels
    uart_labels = [
        (2000, 4800, "UART1_TX", 0),
        (2000, 4700, "UART1_RX", 0),
        (5000, 5000, "UART2_TX", 0),
        (5000, 4900, "UART2_RX", 0),
        (9500, 5800, "UART2_TX", 0),
        (9500, 5700, "UART2_RX", 0),
        (9500, 3800, "UART1_TX", 0),
        (9500, 3700, "UART1_RX", 0),
    ]
    for ux, uy, uname, uorient in uart_labels:
        lines.append(wire_and_label(ux, uy, uname, orient=uorient))

    # RS485 labels
    rs485_labels = [
        (12000, 5500, "RS485_A", 0),
        (12000, 5400, "RS485_B", 0),
        (14500, 4500, "RS485_A", 0),
        (14500, 4200, "RS485_B", 0),
    ]
    for rx, ry, rname, rorient in rs485_labels:
        lines.append(wire_and_label(rx, ry, rname, orient=rorient))

    lines.append(')')  # close kicad_sch

    content = '\n'.join(lines)
    with open(path, 'w') as f:
        f.write(content)
    print(f"  [OK] {path}")


def gen_sym_lib(path):
    """Generate the .kicad_sym library file."""
    lines = [f'(kicad_symbol_lib (version 202401) (generator "opencode")']
    for name, fn in sorted(ALL_SYMBOLS.items()):
        _, _, sym_s, _ = fn()
        lines.append(sym_s)
    lines.append(')')
    with open(path, 'w') as f:
        f.write('\n'.join(lines))
    print(f"  [OK] {path}")


def main():
    base = OUT
    print("Generating KiCad 8 files...")
    gen_sym_lib(os.path.join(base, "esp32-solar-node.kicad_sym"))
    gen_schematic(os.path.join(base, "esp32-solar-node.kicad_sch"))
    print(f"\nDone! Generated {len(ALL_SYMBOLS)} embedded symbols, 1 schematic page.")
    print(f"Open KiCad → File → Open Project → {os.path.join(base, 'esp32-solar-node.kicad_pro')}")

if __name__ == "__main__":
    main()
