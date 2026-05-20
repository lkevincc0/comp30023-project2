use core::fmt;
use std::ffi::{CString, c_char, c_void};
use std::sync::atomic::{AtomicBool, Ordering};

/// The direction a ship extends towards.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Horizontal = 0,
    Vertical = 1,
}

impl fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Direction::Horizontal => write!(f, "Horizontal"),
            Direction::Vertical => write!(f, "Vertical"),
        }
    }
}

/// Representation of a ship.
#[derive(Debug, PartialEq, Eq)]
pub struct Ship {
    /// Coordinate of ship's starting position (e.g. "A1").
    pub coordinate: String,
    /// Length of ship in units (2, 3, 4, 5).
    pub length: u8,
    /// Direction of ship (Horizontal or Vertical).
    pub direction: Direction,
}

/// Outcome of a move.
#[repr(C)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[allow(unused)]
pub enum TurnResult {
    Miss = 0,
    Hit = 1,
    Win = 2,
    Sunk2 = 4,
    Sunk3 = 5,
    Sunk4 = 6,
    Sunk5 = 7,
    Invalid = -1,
}

impl fmt::Display for TurnResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TurnResult::Miss => write!(f, "Miss"),
            TurnResult::Hit => write!(f, "Hit"),
            TurnResult::Win => write!(f, "Win"),
            TurnResult::Sunk2 => write!(f, "Sunk ship of length 2"),
            TurnResult::Sunk3 => write!(f, "Sunk ship of length 3"),
            TurnResult::Sunk4 => write!(f, "Sunk ship of length 4"),
            TurnResult::Sunk5 => write!(f, "Sunk ship of length 5"),
            TurnResult::Invalid => write!(f, "Invalid move"),
        }
    }
}

/// Ensure that the engine is only initialised once.
static ENGINE_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Game Engine
pub struct Engine {
    engine: *mut EngineOpaque,
}

unsafe impl Send for Engine {}

impl Engine {
    /// Initialise game engine.
    /// `None` will be returned if engine fails to initialise or there's already an initialised instance.
    pub fn new() -> Option<Self> {
        if ENGINE_INITIALIZED
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return None;
        }

        let engine = unsafe { engine_init() };
        if engine.is_null() {
            ENGINE_INITIALIZED.store(false, Ordering::Release);
            return None;
        }
        Some(Engine { engine })
    }

    /// Raw engine pointer, for use by signal handlers that need to call `engine_free`
    /// without going through `Drop`.
    pub fn as_raw_ptr(&self) -> *mut c_void {
        self.engine as *mut c_void
    }
}

/// Free an engine via its raw pointer (signal-handler safe).
pub unsafe fn raw_engine_free(p: *mut c_void) {
    unsafe { engine_free(p as *mut EngineOpaque) };
}

impl Engine {
    /// Initialise a game with the given ID.
    ///
    /// * `game_id` - ID of game.
    ///
    /// Returns true on success, false on failure (e.g. game is already initialised).
    pub fn init_game(&mut self, game_id: u32) -> bool {
        unsafe { engine_init_game(self.engine, game_id) }
    }

    /// End a game with the given ID.
    /// If the specified game has not been initialised, this is a no-op.
    ///
    /// * `game_id` - ID of game.
    pub fn end_game(&mut self, game_id: u32) {
        unsafe { engine_end_game(self.engine, game_id) };
    }

    /// Place ships.
    /// The first call for a particular game will place ships for player 1,
    /// and the second call will place ships for player 2.
    ///
    /// * `game_id` - ID of game.
    /// * `ships` - Array of 4 ships to place for the current player. The caller retains ownership of the array.
    ///
    /// Returns player number (1 or 2) on success, or -1 on failure.
    pub fn place_ships(&mut self, game_id: u32, ships: &[Ship; 4]) -> i8 {
        let ship_coordinates: [CString; 4] =
            std::array::from_fn(|i| convert_coordinate(&ships[i].coordinate));
        let c_ships: [CShip; 4] = std::array::from_fn(|i| CShip {
            coordinate: ship_coordinates[i].as_ptr(),
            length: ships[i].length,
            direction: ships[i].direction,
        });

        unsafe { engine_place_ships(self.engine, game_id, &c_ships) }
    }

    /// Take a turn.
    ///
    /// * `game_id` - ID of game.
    /// * `player_number` - Player number of player making the move.
    /// * `coordinate` - Coordinate to attack. The caller retains ownership of the string.
    ///
    /// Returns the result of the turn, or `TurnResult::Invalid` on failure.
    pub fn take_turn(&mut self, game_id: u32, player_number: u8, coordinate: &str) -> TurnResult {
        unsafe {
            engine_take_turn(
                self.engine,
                game_id,
                player_number,
                convert_coordinate(coordinate).as_ptr(),
            )
        }
    }

    /// If the client requests for extended move encoding, this function should be
    /// called instead of `engine_take_turn`. They have the same parameters.
    ///
    /// Returns variable length data, which the engine on the client side will interpret.
    pub fn take_turn_extended(
        &mut self,
        game_id: u32,
        player_number: u8,
        coordinate: &str,
    ) -> Box<[u8]> {
        let extended_result = unsafe {
            engine_take_turn_extended(
                self.engine,
                game_id,
                player_number,
                convert_coordinate(coordinate).as_ptr(),
            )
        };

        if extended_result.length == 0 || extended_result.data.is_null() {
            unsafe { engine_free_extended_result(extended_result) };
            return Box::new([]);
        }

        // Copy the data into a Rust-owned buffer
        let data_slice = unsafe {
            std::slice::from_raw_parts(
                extended_result.data as *const u8,
                extended_result.length as usize,
            )
        };
        let data_vec = data_slice.to_vec();

        // Free the ExtendedTurnResult data allocated by the engine
        unsafe { engine_free_extended_result(extended_result) };

        data_vec.into_boxed_slice()
    }

    /// Extracts a `TurnResult` from variable length data representing an `ExtendedTurnResult`.
    ///
    /// * `extended_result` - The variable length data to extract from.
    pub fn extract_turn_result(&self, extended_result: &Box<[u8]>) -> TurnResult {
        // Convert it back to an ExtendedTurnResult
        let data_len = extended_result.len() as u16;
        let data_ptr = extended_result.as_ptr() as *const c_void;
        let extended_result_ffi = ExtendedTurnResult {
            length: data_len,
            data: data_ptr,
        };

        // Extract with engine
        unsafe { engine_extract_turn_result(extended_result_ffi) }
    }
}

impl Drop for Engine {
    /// On drop, free the FFI engine.
    fn drop(&mut self) {
        unsafe { engine_free(self.engine) };
        ENGINE_INITIALIZED.store(false, Ordering::Release);
    }
}

/// Convert a Rust string coordinate (e.g. "A1") to a C string coordinate.
fn convert_coordinate(rust_str: &str) -> CString {
    CString::new(rust_str.replace('\0', "(NUL)"))
        .expect("Failed to convert internal coordinate to C string")
}

/// Extended result payload, from `engine_take_turn_extended`.
#[repr(C)]
struct ExtendedTurnResult {
    /// Length of data in bytes.
    pub length: u16,
    /// Pointer to data. This can be a NULL pointer.
    pub data: *const c_void,
}

#[repr(C)]
struct EngineOpaque {
    _data: (),
    _marker: core::marker::PhantomData<(*mut u8, core::marker::PhantomPinned)>,
}

/// Ship representation as a C interface.
#[repr(C)]
struct CShip {
    pub coordinate: *const c_char,
    pub length: u8,
    pub direction: Direction,
}

unsafe extern "C" {
    fn engine_init() -> *mut EngineOpaque;
    fn engine_init_game(engine: *mut EngineOpaque, game_id: u32) -> bool;
    fn engine_end_game(engine: *mut EngineOpaque, game_id: u32);
    fn engine_place_ships(engine: *mut EngineOpaque, game_id: u32, ships: &[CShip; 4]) -> i8;
    fn engine_take_turn(
        engine: *mut EngineOpaque,
        game_id: u32,
        player_number: u8,
        coordinate: *const c_char,
    ) -> TurnResult;
    fn engine_take_turn_extended(
        engine: *mut EngineOpaque,
        game_id: u32,
        player_number: u8,
        coordinate: *const c_char,
    ) -> ExtendedTurnResult;
    fn engine_extract_turn_result(extended_result: ExtendedTurnResult) -> TurnResult;
    fn engine_free_extended_result(result: ExtendedTurnResult);
    fn engine_free(engine: *mut EngineOpaque);
}
