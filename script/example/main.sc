{
    @syntax("card","test:string,test2:string;");

   let file = @file();
    let line = @line();
    let column = @column();
    let system_func = @func_lists();
    let content = ["-- script info --","file: "+file.@to_string(),"line: "+line.@to_string(),"column: "+column.@to_string()];
    @write_file("./test-dir/test.txt",content);
    @syntax("card","test:string,test2:string;");
    let a = 0;
    // 登録済みシステム関数情報を読み込み
    //let read_system_func = @read_file("./test-dir/test.txt");
    
    let out =@cmd("mpv",["C:/Users/tanukimaru/Downloads/sounds/*"]);

}
