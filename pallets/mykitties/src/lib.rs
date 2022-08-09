#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{pallet_prelude::{*, DispatchResult}, weights::DispatchClass};
	use frame_system::{pallet_prelude::*, Origin};
	use frame_support::{
		sp_runtime::traits::Hash,
		traits::{ Randomness, Currency, tokens::ExistenceRequirement },
		transactional
	};
	use sp_io::hashing::blake2_128;
	use scale_info::TypeInfo;

	#[cfg(feature = "std")]
	use sp_core::serde::{Deserialize, Serialize};

	// ACTION #1: Write a Struct to hold Kitty information.
	type AccountOf<T> = <T as frame_system::Config>::AccountId;
	type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	#[derive(Clone, Encode, Decode, PartialEq, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct Kitty<T: Config> {
		pub dna: [u8; 16],   
		pub price: Option<BalanceOf<T>>,
		pub gender: Gender,
		pub owner: AccountOf<T>,
	}

	// ACTION #2: Enum declaration for Gender.
	#[derive(Encode, Decode, Debug, Clone, PartialEq, TypeInfo)]
	pub enum Gender {
		Male,
		Female,
	}

	// ACTION #3: Implementation to handle Gender type in Kitty struct.
	impl Default for Gender {
		fn default() -> Self {
			Gender::Male
		}
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	#[pallet::generate_store(trait Store)]
	#[derive(TypeInfo)]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types it depends on.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The Currency handler for the Kitties pallet.
		type Currency: Currency<Self::AccountId>;

		// ACTION #5: Specify the type for Randomness we want to specify for runtime.
		type KittyRandomness: Randomness<Self::Hash, Self::BlockNumber>;

		// ACTION #9: Add MaxKittyOwned constant
		#[pallet::constant]
		type MaxKittyOwned: Get<u32>;
	}

	// Errors.
	#[pallet::error]
	pub enum Error<T> {
		// TODO Part III
		/// Handles arithemtic overflow when incrementing the Kitty counter.
		KittyCntOverflow,
		/// An account cannot own more Kitties than `MaxKittyCount`.
		ExceedMaxKittyOwned,
		/// Buyer cannot be the owner.
		BuyerIsKittyOwner,
		/// Cannot transfer a kitty to its owner.
		TransferToSelf,
		/// Handles checking whether the Kitty exists.
		KittyNotExist,
		/// Handles checking that the Kitty is owned by the account transferring, buying or setting a price for it.
		NotKittyOwner,
		/// Ensures the Kitty is for sale.
		KittyNotForSale,
		/// Ensures that the buying price is greater than the asking price.
		KittyBidPriceTooLow,
		/// Ensures that an account has enough funds to purchase a Kitty. 
		NotEnoughBalance,
	}

	// Events.
	#[pallet::event]
	// #[pallzet::metadata(T::AccountId = "AccountId")]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		// TODO Part III
		// Success(T::Time, T::Day),
		Created(T::AccountId, T::Hash),
		PriceSet(T::AccountId, T::Hash, Option<BalanceOf<T>>),
		Transferred(T::AccountId, T::AccountId, T::Hash),
		Bought(T::AccountId, T::AccountId, T::Hash, BalanceOf<T>),
	}

	#[pallet::storage]
	#[pallet::getter(fn all_kitties_count)]
	pub(super) type KittyCnt<T: Config> = StorageValue<_, u64, ValueQuery>;

	// ACTION #8: Remaining storage items.
	#[pallet::storage]
	#[pallet::getter(fn kitty)]
	pub(super) type Kitties<T: Config> = StorageMap<
		_, 
		Twox64Concat, 
		T::Hash, 
		Kitty<T>
	>;

	#[pallet::storage]
	#[pallet::getter(fn kitties_owned)]
	pub(super) type KittiesOwned<T: Config> = StorageMap<
		_, 
		Twox64Concat, 
		T::AccountId, 
		BoundedVec<T::Hash, T::MaxKittyOwned>,
		ValueQuery
	>;
	// TODO Part IV: Our pallet's genesis configuration.
	// #[pallet::genesis_config]
	// pub struct GenesisConfig<T: Config> {
	// 	pub kitty: Vec<(T::AccountId, [u8; 16], Gender)>,
	// }

	// #[cfg(feature = "std")]
	// impl<T: Config> Default for GenesisConfig<T> {
	// 	fn default() -> GenesisConfig<T> {
	// 		GenesisConfig { kitty: vec![] }
	// 	}
	// }

	// #[pallet::genesis_build]
	// impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
	// 	fn build(&self) {
	// 		// When building a kitty from genesis config, we require the dna and gender to be supplied.
	// 		for (acct, dna, gender) in &self.kitty {
	// 			let _ = <Pallet<T>>::mint(acct, Some(dna.clone()), Some(gender.clone()));
	// 		}
	// 	}
	// }

	#[pallet::call]
	impl<T: Config> Pallet<T> {

		// TODO Part III: create_kitty
		#[pallet::weight(100)]
		pub fn create_kitty(origin: OriginFor<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			let kitty_id = Self::mint(&sender, None, None)?;

			// Logging to the console
			log::info!("A kitty is born with ID: {:?}.", kitty_id);

			// Deposit our "Created" event.
			Self::deposit_event(Event::Created(sender, kitty_id));
			
			Ok(())
		}


		// TODO Part IV: set_price
		#[pallet::weight(100)]
		pub fn set_price(
			origin: OriginFor<T>,
			kitty_id: T::Hash,
			new_price: Option<BalanceOf<T>>
		) -> DispatchResult{
			let sender = ensure_signed(origin)?;

			// ACTION #1: Checking Kitty owner
			ensure!(Self::is_kitty_owner(&kitty_id, &sender)?, <Error<T>>::NotKittyOwner);

			let mut kitty = Self::kitty(&kitty_id).ok_or(<Error<T>>::KittyNotExist)?;

			// ACTION #2: Set the Kitty price and update new Kitty infomation to storage.
			kitty.price = new_price.clone();
			<Kitties<T>>::insert(&kitty_id, kitty);

     		// ACTION #3: Deposit a "PriceSet" event.
			Self::deposit_event(Event::PriceSet(sender, kitty_id, new_price));

			Ok(())
		}

		// TODO Part IV: transfer
		#[pallet::weight(100)]
		pub fn transfer(
			origin: OriginFor<T>,
			to: T::AccountId,
			kitty_id: T::Hash
		) -> DispatchResult {
			let from = ensure_signed(origin)?;

			ensure!(Self::is_kitty_owner(&kitty_id, &from)?, <Error<T>>::NotKittyOwner);

			ensure!(from != to, <Error<T>>::TransferToSelf);

			let to_owned = <KittiesOwned<T>>::get(&to);
			ensure!((to_owned.len() as u32) < T::MaxKittyOwned::get(), <Error<T>>::ExceedMaxKittyOwned);

			Self::transfer_kitty_to(&kitty_id, &to)?;
			Self::deposit_event(Event::Transferred(from, to, kitty_id));


			Ok(())
		}

		// TODO Part IV: buy_kitty
		#[transactional]
		#[pallet::weight(100)]
		pub fn buy_kitty(
			origin: OriginFor<T>,
			kitty_id: T::Hash,
			bid_price: BalanceOf<T>
		) -> DispatchResult {
			let buyer = ensure_signed(origin)?;

			// Check the kitty exists and buyer is not the current kitty owner
			let kitty = Self::kitty(&kitty_id).ok_or(<Error<T>>::KittyNotExist)?;
			ensure!(kitty.owner != buyer, <Error<T>>::BuyerIsKittyOwner);

			// ACTION #7: Check if the Kitty is for sale.
			if let Some(ask_price) = kitty.price {
				ensure!(ask_price <= bid_price, <Error<T>>::KittyBidPriceTooLow);
			} else{
				Err(<Error<T>>::KittyNotForSale)?;
			}

			ensure!(T::Currency::free_balance(&buyer) >= bid_price, <Error<T>>::NotEnoughBalance);

			let to_owned = <KittiesOwned<T>>::get(&buyer);
			ensure!((to_owned.len() as u32) < T::MaxKittyOwned::get(), <Error<T>>::ExceedMaxKittyOwned);

			let seller = kitty.owner.clone();


			// ACTION #8: Update Balances using the Currency trait.
			T::Currency::transfer(&buyer, &seller, bid_price, ExistenceRequirement::KeepAlive)?;

			Self::transfer_kitty_to(&kitty_id, &buyer)?;

			Self::deposit_event(Event::Bought(buyer, seller, kitty_id, bid_price));

			Ok(())
		}


		// TODO Part IV: breed_kitty
		#[pallet::weight(100)]
		pub fn breed_kitty(
			origin: OriginFor<T>,
			kid1: T::Hash,
			kid2: T::Hash
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			// Check: Verify `sender` owns both kitties (and both kitties exist).
			ensure!(Self::is_kitty_owner(&kid1, &sender)?, <Error<T>>::NotKittyOwner);
			ensure!(Self::is_kitty_owner(&kid2, &sender)?, <Error<T>>::NotKittyOwner);

			// ACTION #10: Breed two Kitties using unique DNA
			let new_dna = Self::breed_dna(&kid1, &kid2)?;

			// ACTION #11: Mint new Kitty using new DNA
			Self::mint(&sender, Some(new_dna), None)?;

			Ok(())
		}
	}

	//** Our helper functions.**//

	impl<T: Config> Pallet<T> {

		// ACTION #4: helper function for Kitty struct
		fn gen_gender() -> Gender {
			let random = T::KittyRandomness::random(&b"gender"[..]).0;
			match random.as_ref()[0] % 2 {
				0 => Gender::Male,
				_ => Gender::Female,
			}
		}

		// TODO Part III: helper functions for dispatchable functions

		// ACTION #7: funtion to randomly generate DNA
		fn gen_dna() -> [u8; 16] {
			let payload = (
				T::KittyRandomness::random(&b"dna"[..]).0,
				<frame_system::Pallet<T>>::block_number(),
			);
			payload.using_encoded(blake2_128)
		}

		pub fn breed_dna(kid1: &T::Hash, kid2: &T::Hash) -> Result<[u8; 16], Error<T>> {
			let dna1 = Self::kitty(kid1).ok_or(<Error<T>>::KittyNotExist)?.dna;
			let dna2 = Self::kitty(kid2).ok_or(<Error<T>>::KittyNotExist)?.dna;
	  
			let mut new_dna = Self::gen_dna();
			for i in 0..new_dna.len() {
			  new_dna[i] = (new_dna[i] & dna1[i]) | (!new_dna[i] & dna2[i]);
			}
			Ok(new_dna)
		}

		// TODO Part III: mint
		// Helper to mint a Kitty.
		pub fn mint(
			owner: &T::AccountId,
			dna: Option<[u8; 16]>,
			gender: Option<Gender>,
		) -> Result<T::Hash, Error<T>> {
			let kitty = Kitty::<T> {
				dna: dna.unwrap_or_else(Self::gen_dna),
				price: None,
				gender: gender.unwrap_or_else(Self::gen_gender),
				owner: owner.clone(),
			};
		
			let kitty_id = T::Hashing::hash_of(&kitty);
		
			// Performs this operation first as it may fail
			let new_cnt = Self::all_kitties_count().checked_add(1)
			.ok_or(<Error<T>>::KittyCntOverflow)?;
		
			// Performs this operation first because as it may fail
			<KittiesOwned<T>>::try_mutate(&owner, |kitty_vec| {
			kitty_vec.try_push(kitty_id)
			}).map_err(|_| <Error<T>>::ExceedMaxKittyOwned)?;
		
			<Kitties<T>>::insert(kitty_id, kitty);
			<KittyCnt<T>>::put(new_cnt);
			Ok(kitty_id)
		}

		pub fn is_kitty_owner(kitty_id: &T::Hash, acct: &T::AccountId) -> Result<bool, Error<T>>{
			match Self::kitty(kitty_id){
				Some(kitty) => Ok(kitty.owner == *acct),
				None => Err(<Error<T>>::KittyNotExist)
			}
		}

		// TODO Part IV: transfer_kitty_to
		// #[transactional]
		pub fn transfer_kitty_to(
			kitty_id: &T::Hash,
			to: &T::AccountId,
		) -> Result<(), Error<T>>{
			let mut kitty = Self::kitty(&kitty_id).ok_or(<Error<T>>::KittyNotExist)?;
			let prev_owner = kitty.owner.clone();

			<KittiesOwned<T>>::try_mutate(&prev_owner, |owned|{
				if let Some(ind) = owned.iter().position(|&id| id == *kitty_id){
					owned.swap_remove(ind);
					return Ok(());
				}
				Err(())
			}).map_err(|_| <Error<T>>::KittyNotExist)?;

			kitty.owner = to.clone();
			kitty.price = None;

			<Kitties<T>>::insert(kitty_id, kitty);

			<KittiesOwned<T>>::try_mutate(to, |vec|{
				vec.try_push(*kitty_id)
			}).map_err(|_| <Error<T>>::ExceedMaxKittyOwned)?;

			Ok(())
		}
	}
}
