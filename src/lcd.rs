/*
********************************************************************
The following is a derivative of the Arduino LiquidCrystal library.
********************************************************************
https://github.com/arduino-libraries/LiquidCrystal

Copyright © 2006-2008 Hans-Christoph Steiner. All rights reserved.
Copyright (c) 2010 Arduino LLC. All right reserved.

This library is free software; you can redistribute it and/or modify it
under the terms of the GNU Lesser General Public License as published
by the Free Software Foundation; either version 2.1 of the License,
or (at your option) any later version.

This library is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
See the GNU Lesser General Public License for more details.

You should have received a copy of the
GNU Lesser General Public License along with this library;
if not, write to the Free Software Foundation, Inc.,
51 Franklin St, Fifth Floor, Boston, MA 02110-1301 USA
*/

use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_sys::gpio_set_level;
use esp_idf_sys::gpio_set_direction;
use esp_idf_sys::gpio_mode_t_GPIO_MODE_OUTPUT;
use crate::lcd;

const LCD_CLEARDISPLAY: u8 = 0x01;
const LCD_ENTRYMODESET: u8 = 0x04;
const LCD_DISPLAYCONTROL: u8 = 0x08;
const LCD_FUNCTIONSET: u8 = 0x20;
const LCD_SETDDRAMADDR: u8 = 0x80;

// flags for display entry mode
const LCD_ENTRYLEFT: u8 = 0x02;
const LCD_ENTRYSHIFTDECREMENT: u8 = 0x00;

// flags for display on/off control
const LCD_DISPLAYON: u8 = 0x04;
const LCD_CURSOROFF: u8 = 0x00;
const LCD_BLINKOFF: u8 = 0x00;

// flags for function set
const LCD_4BITMODE: u8 = 0x00;
const LCD_2LINE: u8 = 0x08;
const LCD_5X8DOTS: u8 = 0x00;

static mut RS_PIN: i32 = 0; // LOW: command. HIGH: character.
static mut EN_PIN: i32 = 0; // activated by a HIGH pulse.
static mut DATA_PINS: [i32; 8] = [0; 8];

static mut DISPLAY_FUNCTION: u8 = 0;
static mut DISPLAY_CONTROL: u8 = 0;
static mut DISPLAY_MODE: u8 = 0;
static mut ROW_OFFSETS: [i32; 4] = [0; 4];

fn micros_to_millis(micros: u64) -> u32 {
    (micros / 1000) as u32
}

pub unsafe fn init(cols: u8, rs: i32, en: i32, d0: i32, d1: i32, d2: i32, d3: i32) {
    RS_PIN = rs;
    EN_PIN = en;

    DATA_PINS[0] = d0;
    DATA_PINS[1] = d1;
    DATA_PINS[2] = d2;
    DATA_PINS[3] = d3;

    DISPLAY_FUNCTION = LCD_4BITMODE | LCD_2LINE | LCD_5X8DOTS;

    set_row_offsets(0x00, 0x40, (0x00 + cols).into(), (0x40 + cols).into());
    FreeRtos::delay_ms(micros_to_millis(1000));

    gpio_set_direction(RS_PIN, gpio_mode_t_GPIO_MODE_OUTPUT);
    gpio_set_direction(EN_PIN, gpio_mode_t_GPIO_MODE_OUTPUT);

    for i in 0..4 {
        gpio_set_direction(DATA_PINS[i], gpio_mode_t_GPIO_MODE_OUTPUT);
    }

    // SEE PAGE 45/46 FOR INITIALIZATION SPECIFICATION!
    // according to datasheet, we need at least 40 ms after power rises above 2.7 V
    // before sending commands. Arduino can turn on way before 4.5 V so we'll wait 50
    FreeRtos::delay_ms(micros_to_millis(50000));

    // Now we pull both RS and R/W low to begin commands
    gpio_set_level(RS_PIN, 0);
    gpio_set_level(EN_PIN, 0);

    // this is according to the Hitachi HD44780 datasheet
    // figure 24, pg 46
    // we start in 8bit mode, try to set 4 bit mode
    write4bits(0x03);
    FreeRtos::delay_ms(micros_to_millis(4500)); // wait min 4.1ms

    // second try
    write4bits(0x03);
    FreeRtos::delay_ms(micros_to_millis(4500)); // wait min 4.1ms

    // third go!
    write4bits(0x03);
    FreeRtos::delay_ms(micros_to_millis(150));

    // finally, set to 4-bit interface
    write4bits(0x02);

    // finally, set # lines, font size, etc.
    command(LCD_FUNCTIONSET | DISPLAY_FUNCTION);

    // turn the display on with no cursor or blinking default
    DISPLAY_CONTROL = LCD_DISPLAYON | LCD_CURSOROFF | LCD_BLINKOFF;
    display();

    // clear it off
    clear();

    // Initialize to default text direction (for romance languages)
    DISPLAY_MODE = LCD_ENTRYLEFT | LCD_ENTRYSHIFTDECREMENT;
    // set the entry mode
    command(LCD_ENTRYMODESET | DISPLAY_MODE);
}

unsafe fn set_row_offsets(row0: i32,  row1: i32,  row2: i32,  row3: i32)
{
    ROW_OFFSETS[0] = row0;
    ROW_OFFSETS[1] = row1;
    ROW_OFFSETS[2] = row2;
    ROW_OFFSETS[3] = row3;
}

/********** high level commands, for the user! */
pub unsafe fn clear()
{
    command(LCD_CLEARDISPLAY);  // clear display, set cursor position to zero
    FreeRtos::delay_ms(micros_to_millis(2000));  // this command takes a long time!
}

pub unsafe fn set_cursor(col: u8, row: usize)
{
    command(LCD_SETDDRAMADDR | (col + ROW_OFFSETS[row] as u8));
}

unsafe fn display() {
    DISPLAY_CONTROL |= LCD_DISPLAYON;
    command(LCD_DISPLAYCONTROL | DISPLAY_CONTROL);
}

/*********** mid level commands, for sending data/cmds */

unsafe fn command(value: u8) {
    send(value, 0);
}

pub unsafe fn write(value: u8) {
    send(value, 1);
}


pub unsafe fn text(str:String){

    for c in str.chars() {
            lcd::write(c as u8);
    }


}



/************ low level data pushing commands **********/

// write either command or data, with automatic 4/8-bit selection
unsafe fn send(value: u8, mode: u8) {
    gpio_set_level(RS_PIN, mode as u32);
    write4bits(value>>4);
    write4bits(value);
}

unsafe fn pulse_enable() {
    gpio_set_level(EN_PIN, 0);
    FreeRtos::delay_ms(micros_to_millis(1));
    gpio_set_level(EN_PIN, 1);
    FreeRtos::delay_ms(micros_to_millis(1));    // enable pulse must be >450 ns
    gpio_set_level(EN_PIN, 0);
    FreeRtos::delay_ms(micros_to_millis(100));   // commands need >37 us to settle
}

unsafe fn write4bits(value: u8) {
    for i in 0..4 {
        gpio_set_level(DATA_PINS[i], ((value >> i) & 0x01) as u32);
        FreeRtos::delay_ms(micros_to_millis(1));
    }

    pulse_enable();
}
