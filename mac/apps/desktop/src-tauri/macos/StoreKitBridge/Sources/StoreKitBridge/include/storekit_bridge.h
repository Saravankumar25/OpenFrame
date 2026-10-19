/*
 * C interface of the OpenFrame StoreKit 2 bridge (Mac App Store build only).
 *
 * NOT COMPILED YET: written on Windows. build.rs compiles the package on macOS. See Package.swift.
 *
 * Every function that returns `char *` returns a NUL-terminated UTF-8 JSON object allocated with
 * strdup. The caller owns it and must release it with storekit_free exactly once. A successful
 * answer has "ok": true; a failure is {"ok": false, "error": "<code>"}.
 *
 * Every call blocks until StoreKit answers or a timeout passes, and refuses to run on the main
 * thread (error "main_thread"). Call from a background thread only.
 *
 * Only transactions that StoreKit reports as verified are serialised. Unverified ones are counted
 * in "unverifiedCount" and never passed on. No function compares dates or decides entitlement.
 */

#ifndef OPENFRAME_STOREKIT_BRIDGE_H
#define OPENFRAME_STOREKIT_BRIDGE_H

#ifdef __cplusplus
extern "C" {
#endif

/* {"ok":true,"transactions":[Transaction...],"unverifiedCount":n,"skippedCount":n}
 * Transaction: {"productID","id","originalID","appTransactionID"?,"purchaseDateMs",
 *               "revocationDateMs"?,"environment","jwsRepresentation"} */
char *storekit_current_entitlements(void);

/* product_id must be one of the two OpenFrame product ids (at most 4096 bytes).
 * {"ok":true,"outcome":"purchased","transaction":Transaction}
 * {"ok":true,"outcome":"pending"}     Ask to Buy or another deferred step
 * {"ok":true,"outcome":"cancelled"}   the user closed the sheet
 * {"ok":true,"outcome":"unverified"}  StoreKit could not verify the result
 * {"ok":true,"outcome":"unknown"}     a result this version does not know */
char *storekit_purchase(const char *product_id);

/* {"ok":true,"verified":bool,"appTransactionID"?,"environment"?,"jwsRepresentation"?} */
char *storekit_app_transaction(void);

/* {"ok":true,"environment":"Production"|"Sandbox"|"Xcode"} from the verified app transaction. */
char *storekit_environment(void);

/* product_ids_json: a JSON array of product ids (at most 4096 bytes). Unknown ids are dropped.
 * {"ok":true,"products":[{"id","displayName","displayPrice"}]}  for display only */
char *storekit_products(const char *product_ids_json);

/* Starts one listener on Transaction.updates and one on Storefront.updates. For each update the
 * callback receives {"kind":"transaction","transaction":Transaction}, {"kind":"unverified"} or
 * {"kind":"storefront"} (the App Store storefront changed, for example another Apple Account
 * signed in). The callback runs on a StoreKit task, never the main thread.
 * The string is valid only during the callback; do not free it. Returns 1 when the listener runs
 * (also when it was already running), 0 when callback is NULL. */
typedef void (*storekit_update_callback)(const char *json);
int storekit_start_updates(storekit_update_callback callback);

/* Releases a string returned by any function above. NULL is accepted. */
void storekit_free(char *ptr);

#ifdef __cplusplus
}
#endif

#endif /* OPENFRAME_STOREKIT_BRIDGE_H */
