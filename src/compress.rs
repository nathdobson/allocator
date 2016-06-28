//trait Reference<'a> {
//    type Target;
//    fn decompress(Self) -> &'a Self::Target;
//    fn compress(&'a Self::Target) -> Self;
//}
//impl<'a, T> Reference<'a> for &'a T {
//    fn decompress(x: Self) -> &'a Target {
//        return x;
//    }
//    fn compress(x: &'a Target) -> Self {
//        return x;
//    }
//}
//struct SingletonReference<'a,T>(PhantomData<&'a T>);
//static SINGLETON : i32 = 0;
//impl <'a,T> Reference for SingletonReference<'a,T>{
//    fn decompress(x: Self) -> &'a Target {
//        return x;
//    }
//    fn compress(x: &'a Target) -> Self {
//        return x;
//    }
//}