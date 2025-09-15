# X448

This is a pure Rust implementation of the Diffie hellman key exchange protocol, X448. 

## Example 

In the following example, Alice has an Ephemeral(One-Time use) Secret key and Bob has a Static(Multiple use) Key.

### Setup Phase

In this stage, both parties generate a secret and send the corresponding public key to the other party. 

#### Alice

- Alice generates a new one-time use secret key using randomness. 
- Alice sends her public key to Bob, so that Bob can compute the shared key.

```rust
        let alice_secret = Secret::new(&mut OsRng);
        let alice_public_key = PublicKey::from(&alice_secret);
```

#### Bob

- Bob loads a secret key from his storage.
- Bob sends her public key to Alice, so that Alice can compute the shared key.

```rust
        let bob_secret = Secret::from_bytes(bytes_of_key);
        let bob_public_key = PublicKey::from(&bob_secret);
```

### Key Exchange Phase

Both parties now have enough information to compute the shared key. They now independently compute the shared key.

#### Alice
```rust
    let alices_shared_secret = alice_secret.to_diffie_hellman(&bob_public_key);
```
#### Bob
```rust
    let bobs_shared_secret = bob_secret.as_diffie_hellman(&alice_public_key);
```

That's it. Kinda.

- If neither Alice or Bob used a low order point, then they will both have the same shared_key. If one party, did indeed manage to circumvent the API and the honest party tries to use this low order point in a Diffie-hellman key exchange, the shared key will return None.

- Notice that Alice used a method named `to_diffie_hellman` while Bob used a method named `as_diffie_hellman`. Using `to_diffie_hellman` will consume the Secret key, denoting that the key can only be used once in a Diffie-Hellman Key exchange. Of course, you could copy it before it was consumed, but then it would have been more efficient to use `as_diffie_hellman`. 


## API Discussion

The API exposes 3 structures; a public key , a shared secret key and a secret key.

- The public key is an elliptic curve point. It corresponds to a Scalar that the user owns.
- The secret key is a Scalar. This Scalar corresponds to the user's public key and the generator specified in the RFC.
- The shared secret key is an elliptic curve point. It is the key that is computed by both parties in the diffie-hellman key exchange algorithm.  

### Differences to X25519-Dalek

In our API there is only one Secret Scalar Key. In X25519-Dalek there are two types of secret scalar keys;

- An ephemeral secret key which can only be used once in a Diffie-Hellman key exchange.
- A static secret key which can be used multiple times in a diffie hellman key exchange.

Following the example above, this API achieves this example by using two methods on the same struct `to_diffie_hellman` and `as_diffie_hellman`. The rationale for choosing this API is that there will be less code duplication, as there is only one struct. Furthermore, this API also achieves the compile time guarantee that if the user wants to treat the Secret key as Ephemeral, then it will only be used once. One disadvantage of this API, is the lack of distinction between Secrets. For example, as a consumer of an API it would be advantageous to state that the type I want to accept is an `EphemeralSecret` , this decreases the reading complexity as the type encodes the fact that it can only be used once in an diffie-hellman key exchange.
