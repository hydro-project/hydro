use dfir_rs::dfir_parser;
use multiplatform_test::multiplatform_test;

#[multiplatform_test]
pub fn dfir_cycle_issue() {
    dfir_parser! {
        a1v1 = source_stream (DUMMY_SOURCE) -> map (| res | { let (id , b) = res . unwrap () ; (hydro_lang :: __staged :: location :: MemberId :: < amzn_hydro_project_template :: __staged :: CounterNode > :: from_tagless (id as hydro_lang :: __staged :: location :: TaglessMemberId) , hydro_lang :: runtime_support :: bincode :: deserialize :: < (std :: string :: String , std :: string :: String , i64) > (& b) . unwrap ()) })
            -> map (stageleft :: runtime_support :: fnmut1_type_hint :: < (amzn_hydro_project_template :: __staged :: __deps :: hydro_lang :: location :: member_id :: MemberId < amzn_hydro_project_template :: __staged :: CounterNode > , (std :: string :: String , std :: string :: String , i64)) , (std :: string :: String , std :: string :: String , i64) > ({ use hydro_lang :: __staged :: __deps :: * ; use hydro_lang :: __staged :: live_collections :: keyed_stream :: * ; | (_ , v) | v }))
            -> identity :: < (std :: string :: String , std :: string :: String , i64) > ();
        a3v1 = source_iter ([{ use crate :: __staged :: __deps :: * ; use crate :: __staged :: * ; false }]);
        a4v1 = persist :: < 'static > ();
        a5v1 = singleton();
        // a6v1 = for_each (| _ | { });
        a7v1 = source_stream (__hydro_sidecar_0_stream);
        a8v1 = map (stageleft :: runtime_support :: fnmut1_type_hint :: < (u64 , amzn_hydro_project_template :: __staged :: protocol :: CounterCommand) , ((amzn_hydro_project_template :: __staged :: protocol :: Ingress , u64) , amzn_hydro_project_template :: __staged :: protocol :: CounterCommand) > ({ use crate :: __staged :: __deps :: * ; use crate :: __staged :: * ; | (id , cmd) : (u64 , CounterCommand) | ((Ingress :: Smithy , id) , cmd) }));
        a9v1 = source_stream (__hydro_sidecar_1_stream);
        a10v1 = map (stageleft :: runtime_support :: fnmut1_type_hint :: < (u64 , amzn_hydro_project_template :: __staged :: protocol :: CounterCommand) , ((amzn_hydro_project_template :: __staged :: protocol :: Ingress , u64) , amzn_hydro_project_template :: __staged :: protocol :: CounterCommand) > ({ use crate :: __staged :: __deps :: * ; use crate :: __staged :: * ; | (id , cmd) : (u64 , CounterCommand) | ((Ingress :: Grpc , id) , cmd) }));
        a11v1 = chain ();
        a12v1 = tee ();
        a13v1 = filter_map (stageleft :: runtime_support :: fn1_type_hint :: < ((amzn_hydro_project_template :: __staged :: protocol :: Ingress , u64) , amzn_hydro_project_template :: __staged :: protocol :: CounterCommand) , core :: option :: Option < std :: string :: String > > ({ use crate :: __staged :: __deps :: * ; use crate :: __staged :: * ; | (_cid , cmd) : (ClientKey , CounterCommand) | { match cmd { CounterCommand :: Increment { key , .. } => Some (key) , _ => None , } } }));
        a14v1 = map (stageleft :: runtime_support :: fnmut1_type_hint :: < std :: string :: String , (std :: string :: String , std :: string :: String) > ({ use crate :: __staged :: __deps :: * ; use crate :: __staged :: * ; let CLUSTER_SELF_ID__free = hydro_lang :: __staged :: location :: MemberId :: < amzn_hydro_project_template :: __staged :: CounterNode > :: from_tagless ((__hydro_lang_cluster_self_id_loc1v1) . clone ()) ; move | key : String | { let node_id = CLUSTER_SELF_ID__free . clone () . into_tagless () . to_string () ; (key , node_id) } }));
        a15v1 = map ({ let __hydro_singleton_ref_0 = #a5v1 ; stageleft :: runtime_support :: fnmut1_type_hint :: < (std :: string :: String , std :: string :: String) , ((std :: string :: String , std :: string :: String , i64) , bool) > ({ use crate :: __staged :: __deps :: * ; use crate :: __staged :: * ; let by_two_ref__free = __hydro_singleton_ref_0 ; | t | ((t . 0 , t . 1 , if * by_two_ref__free { 2 } else { 1 }) , true) }) });
        a16v1 = map (stageleft :: runtime_support :: fnmut1_type_hint :: < (std :: string :: String , std :: string :: String , i64) , ((std :: string :: String , std :: string :: String , i64) , bool) > ({ use crate :: __staged :: __deps :: * ; use crate :: __staged :: * ; | t | (t , false) }));
        a17v1 = chain ();
        a18v1 = fold :: < 'static > (stageleft :: runtime_support :: fn0_type_hint :: < std :: rc :: Rc < core :: cell :: RefCell < std :: collections :: hash_map :: HashMap < std :: string :: String , std :: collections :: hash_map :: HashMap < std :: string :: String , i64 > > > > > ({ use crate :: __staged :: __deps :: * ; use crate :: __staged :: * ; | | { std :: rc :: Rc :: new (std :: cell :: RefCell :: new (HashMap :: < String , GCounter > :: new ())) } }) , stageleft :: runtime_support :: fn2_borrow_mut_type_hint :: < std :: rc :: Rc < core :: cell :: RefCell < std :: collections :: hash_map :: HashMap < std :: string :: String , std :: collections :: hash_map :: HashMap < std :: string :: String , i64 > > > > , ((std :: string :: String , std :: string :: String , i64) , bool) , () > ({ use crate :: __staged :: __deps :: * ; use crate :: __staged :: * ; | store , (update , is_local) : ((String , String , i64) , bool) | { let mut map = store . borrow_mut () ; let (key , node_id , amount) = update ; let gc = map . entry (key) . or_default () ; let entry = gc . entry (node_id) . or_insert (0) ; if is_local { * entry += amount ; } else { * entry = (* entry) . max (amount) ; } } }));
        a19v1 = singleton();
        // a20v1 = for_each (| _ | { });
        a21v1 = source_stream (DUMMY);
        a22v1 = map (stageleft :: runtime_support :: fnmut1_type_hint :: < (amzn_hydro_project_template :: __staged :: __deps :: hydro_lang :: location :: member_id :: TaglessMemberId , amzn_hydro_project_template :: __staged :: __deps :: hydro_lang :: location :: MembershipEvent) , (amzn_hydro_project_template :: __staged :: __deps :: hydro_lang :: location :: member_id :: MemberId < amzn_hydro_project_template :: __staged :: CounterNode > , amzn_hydro_project_template :: __staged :: __deps :: hydro_lang :: location :: MembershipEvent) > ({ use hydro_lang :: __staged :: __deps :: * ; use hydro_lang :: __staged :: location :: * ; | (k , v) | (MemberId :: from_tagless (k) , v) }));
        a23v1 = fold_keyed :: < 'static > (stageleft :: runtime_support :: fn0_type_hint :: < bool > ({ use hydro_lang :: __staged :: __deps :: * ; use hydro_lang :: __staged :: live_collections :: stream :: networking :: * ; | | false }) , stageleft :: runtime_support :: fn2_borrow_mut_type_hint :: < bool , amzn_hydro_project_template :: __staged :: __deps :: hydro_lang :: location :: MembershipEvent , () > ({ use hydro_lang :: __staged :: __deps :: * ; use hydro_lang :: __staged :: live_collections :: stream :: networking :: * ; | present , event | { match event { MembershipEvent :: Joined => * present = true , MembershipEvent :: Left => * present = false , } } }));
        a24v1 = filter (stageleft :: runtime_support :: fn1_borrow_type_hint :: < (amzn_hydro_project_template :: __staged :: __deps :: hydro_lang :: location :: member_id :: MemberId < amzn_hydro_project_template :: __staged :: CounterNode > , bool) , bool > ({ use hydro_lang :: __staged :: __deps :: * ; use hydro_lang :: __staged :: live_collections :: keyed_singleton :: * ; let f__free = stageleft :: runtime_support :: fn1_borrow_type_hint :: < bool , bool > ({ use hydro_lang :: __staged :: __deps :: * ; use hydro_lang :: __staged :: live_collections :: stream :: networking :: * ; | b | * b }) ; { let orig = f__free ; move | t : & (_ , _) | orig (& t . 1) } }));
        a25v1 = map (stageleft :: runtime_support :: fnmut1_type_hint :: < (amzn_hydro_project_template :: __staged :: __deps :: hydro_lang :: location :: member_id :: MemberId < amzn_hydro_project_template :: __staged :: CounterNode > , bool) , amzn_hydro_project_template :: __staged :: __deps :: hydro_lang :: location :: member_id :: MemberId < amzn_hydro_project_template :: __staged :: CounterNode > > ({ use hydro_lang :: __staged :: __deps :: * ; use hydro_lang :: __staged :: live_collections :: keyed_singleton :: * ; | (k , _) | k }));
        a26v1 = map ({ let __hydro_singleton_ref_1 = #a19v1 ; stageleft :: runtime_support :: fnmut1_type_hint :: < ((amzn_hydro_project_template :: __staged :: protocol :: Ingress , u64) , amzn_hydro_project_template :: __staged :: protocol :: CounterCommand) , (((amzn_hydro_project_template :: __staged :: protocol :: Ingress , u64) , amzn_hydro_project_template :: __staged :: protocol :: CounterResponse) , core :: option :: Option < (std :: string :: String , std :: string :: String , i64) >) > ({ use crate :: __staged :: __deps :: * ; use crate :: __staged :: * ; let CLUSTER_SELF_ID__free = hydro_lang :: __staged :: location :: MemberId :: < amzn_hydro_project_template :: __staged :: CounterNode > :: from_tagless ((__hydro_lang_cluster_self_id_loc1v1) . clone ()) ; let store_ref__free = __hydro_singleton_ref_1 ; { let node_id = CLUSTER_SELF_ID__free . clone () . into_tagless () . to_string () ; move | (client_id , cmd) | { let map = store_ref__free . borrow () ; let (resp , repl) = crate :: __staged :: handle_command (& node_id , & map , & cmd) ; ((client_id , resp) , repl) } } }) });
        a27v1 = tee ();
        a28v1 = filter_map (stageleft :: runtime_support :: fn1_type_hint :: < (((amzn_hydro_project_template :: __staged :: protocol :: Ingress , u64) , amzn_hydro_project_template :: __staged :: protocol :: CounterResponse) , core :: option :: Option < (std :: string :: String , std :: string :: String , i64) >) , core :: option :: Option < (std :: string :: String , std :: string :: String , i64) > > ({ use crate :: __staged :: __deps :: * ; use crate :: __staged :: * ; | ((_cid , _resp) , repl) : ((ClientKey , CounterResponse) , Option < (String , String , i64) >) | repl }));
        a29v1 = cross_join_multiset :: < 'tick , 'tick > ();
        a30v1 = map (hydro_lang :: runtime_support :: stageleft :: runtime_support :: fn1_type_hint :: < (hydro_lang :: __staged :: location :: MemberId < _ > , (std :: string :: String , std :: string :: String , i64)) , _ > (| (id , data) | { (id . into_tagless () , hydro_lang :: runtime_support :: bincode :: serialize (& data) . unwrap () . into ()) }));
        a31v1 = dest_sink (DUMMY_SINK);
        a36v1 = map (stageleft :: runtime_support :: fnmut1_type_hint :: < (((amzn_hydro_project_template :: __staged :: protocol :: Ingress , u64) , amzn_hydro_project_template :: __staged :: protocol :: CounterResponse) , core :: option :: Option < (std :: string :: String , std :: string :: String , i64) >) , ((amzn_hydro_project_template :: __staged :: protocol :: Ingress , u64) , amzn_hydro_project_template :: __staged :: protocol :: CounterResponse) > ({ use crate :: __staged :: __deps :: * ; use crate :: __staged :: * ; | ((cid , resp) , _repl) : ((ClientKey , CounterResponse) , Option < (String , String , i64) >) | (cid , resp) }));
        a37v1 = tee ();
        a38v1 = filter_map (stageleft :: runtime_support :: fn1_type_hint :: < ((amzn_hydro_project_template :: __staged :: protocol :: Ingress , u64) , amzn_hydro_project_template :: __staged :: protocol :: CounterResponse) , core :: option :: Option < (u64 , amzn_hydro_project_template :: __staged :: protocol :: CounterResponse) > > ({ use crate :: __staged :: __deps :: * ; use crate :: __staged :: * ; | ((ingress , id) , resp) : (ClientKey , CounterResponse) | { if matches ! (ingress , Ingress :: Smithy) { Some ((id , resp)) } else { None } } }));
        a39v1 = identity :: < (u64 , amzn_hydro_project_template :: __staged :: protocol :: CounterResponse) > ();
        a39v1 -> dest_sink (__hydro_sidecar_0_sink);
        a40v1 = filter_map (stageleft :: runtime_support :: fn1_type_hint :: < ((amzn_hydro_project_template :: __staged :: protocol :: Ingress , u64) , amzn_hydro_project_template :: __staged :: protocol :: CounterResponse) , core :: option :: Option < (u64 , amzn_hydro_project_template :: __staged :: protocol :: CounterResponse) > > ({ use crate :: __staged :: __deps :: * ; use crate :: __staged :: * ; | ((ingress , id) , resp) : (ClientKey , CounterResponse) | { if matches ! (ingress , Ingress :: Grpc) { Some ((id , resp)) } else { None } } }));
        a41v1 = identity :: < (u64 , amzn_hydro_project_template :: __staged :: protocol :: CounterResponse) > ();
        a41v1 -> dest_sink (__hydro_sidecar_1_sink);
        a3v1 -> a4v1;
        a4v1 -> a5v1;
        // a5v1 -> a6v1;
        a7v1 -> a8v1;
        a9v1 -> a10v1;
        a8v1 -> a11v1;
        a10v1 -> a11v1;
        a11v1 -> a12v1;
        a12v1 -> a13v1;
        a13v1 -> a14v1;
        a14v1 -> a15v1;
        a1v1 -> a16v1;
        a15v1 -> a17v1;
        a16v1 -> a17v1;
        a17v1 -> a18v1;
        a18v1 -> a19v1;
        // a19v1 -> a20v1;
        a21v1 -> a22v1;
        a22v1 -> a23v1;
        a23v1 -> a24v1;
        a24v1 -> a25v1;
        a12v1 -> a26v1;
        a26v1 -> a27v1;
        a27v1 -> a28v1;
        a25v1 -> [0]a29v1;
        a28v1 -> [1]a29v1;
        a30v1 -> a31v1;
        a29v1 -> a30v1;
        a27v1 -> a36v1;
        a36v1 -> a37v1;
        a37v1 -> a38v1;
        a38v1 -> a39v1;
        a37v1 -> a40v1;
        a40v1 -> a41v1;
    }
}
