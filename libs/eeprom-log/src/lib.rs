#![no_std]

use core::marker::{Copy, PhantomData};

use embedded_hal::blocking::i2c::{Write, WriteRead};

use eeprom24x::{
    eeprom24x::{MultiSizeAddr, PageWrite},
    Eeprom24x, Error,
};

#[derive(Copy, Clone)]
struct StorageItem<T: Copy> {
    pub block_counter: usize,
    pub item: T,
}

pub struct EepromLog<T: Copy, I2C, PS, AS> {
    eeprom: Eeprom24x<I2C, PS, AS>,
    current_block_offset: usize,
    current_block_counter: usize,
    _item: PhantomData<StorageItem<T>>,
}

impl<T: Copy + Default, E, I2C, PS, AS> EepromLog<T, I2C, PS, AS>
where
    I2C: Write<Error = E> + WriteRead<Error = E>,
    AS: MultiSizeAddr,
    E: core::fmt::Debug,
    Eeprom24x<I2C, PS, AS>: PageWrite<E>,
{
    pub fn init(eeprom: Eeprom24x<I2C, PS, AS>) -> Self {
        assert_eq!(256 % core::mem::size_of::<StorageItem<T>>(), 0);
        let mut res = Self {
            eeprom,
            current_block_offset: 0,
            current_block_counter: 0,
            _item: PhantomData,
        };

        let mut prev_block = res.read(0).expect("Failed to access EEPROM");
        if prev_block.block_counter != usize::MAX {
            'search_loop: for item_n in 1.. {
                match res.read(item_n) {
                    Ok(current_block) => {
                        // включая варп 0xffffffff -> 0x0
                        if current_block.block_counter != prev_block.block_counter.wrapping_add(1) {
                            res.current_block_offset = item_n;
                            res.current_block_counter = prev_block.block_counter.wrapping_add(1);
                            break 'search_loop;
                        } else {
                            prev_block = current_block;
                        }
                    }
                    Err(eeprom24x::Error::InvalidAddr) => {
                        // непрерывная последовательность до конца флешки, значит будем писасть с начала.
                        res.current_block_offset = 0;
                        res.current_block_counter = prev_block.block_counter.wrapping_add(1);
                        break 'search_loop;
                    }
                    Err(eeprom24x::Error::I2C(e)) => panic!("i2c error: {:?}", e),
                    Err(eeprom24x::Error::TooMuchData) => unreachable!(),
                }
            }
        } else {
            // current_offset = 0, current_index = 0 - ok!
        }

        res
    }

    fn read(&mut self, offset: usize) -> Result<StorageItem<T>, eeprom24x::Error<E>> {
        let mut res: StorageItem<T> = unsafe { core::mem::MaybeUninit::uninit().assume_init() };

        unsafe {
            let res = core::slice::from_raw_parts_mut(
                &mut res as *mut _ as *mut u8,
                core::mem::size_of::<StorageItem<T>>(),
            );

            self.eeprom.read_data(
                (offset * core::mem::size_of::<StorageItem<T>>()) as u32,
                res,
            )?;

            Ok(*(res.as_ptr() as *const StorageItem<T>))
        }
    }

    pub fn last(&mut self) -> Result<T, eeprom24x::Error<E>> {
        if self.current_block_offset > 0 {
            let current = self.read(self.current_block_offset.wrapping_sub(1))?;
            if current.block_counter != usize::MAX {
                return Ok(current.item)
            }
        }
        Ok(T::default()) // no valid data empty flash
    }

    pub fn append(&mut self, val: T) -> Result<usize, eeprom24x::Error<E>> {
        let mut address =
            (self.current_block_offset * core::mem::size_of::<StorageItem<T>>()) as u32;
        let data = StorageItem::<T> {
            block_counter: self.current_block_counter,
            item: val,
        };

        let data = unsafe {
            core::slice::from_raw_parts(
                &data as *const _ as *const u8,
                core::mem::size_of::<StorageItem<T>>(),
            )
        };

        loop {
            match eeprom24x::eeprom24x::PageWrite::page_write(&mut self.eeprom, address, data) {
                Ok(_) => {
                    self.current_block_counter = self.current_block_counter.wrapping_add(1);
                    self.current_block_offset =
                        (address as usize / core::mem::size_of::<StorageItem<T>>()) + 1;

                    return Ok(address as usize / core::mem::size_of::<StorageItem<T>>());
                }
                Err(Error::I2C(e)) => return Err(Error::I2C(e)),
                Err(Error::TooMuchData) => panic!(),
                Err(Error::InvalidAddr) => {
                    // reset to start of flash
                    address = 0;
                }
            }
        }
    }
}
