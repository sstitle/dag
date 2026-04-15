{
  testAddition = {
    expr = 1 + 1;
    expected = 2;
  };

  testStringConcatenation = {
    expr = "hello" + " " + "world";
    expected = "hello world";
  };

  testListFilter = {
    expr = builtins.filter (x: x > 2) [
      1
      2
      3
      4
    ];
    expected = [
      3
      4
    ];
  };

  testMap = {
    expr = map (x: x * 2) [
      1
      2
      3
    ];
    expected = [
      2
      4
      6
    ];
  };
}
